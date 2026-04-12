// Copyright 2026 the Xilem Authors
// SPDX-License-Identifier: Apache-2.0

//! Reusable `subduction`-backed presentation for Masonry visual layers.
//!
//! This module owns the host-side realization of a [`VisualLayerPlan`] into a single WGPU
//! surface using `subduction`.
//!
//! Scene-layer rendering is a separate responsibility from composition/presentation. This module
//! provides a Vello-backed scene renderer as a default, but hosts can supply their own
//! [`SceneLayerRenderer`] if they want to manage Masonry-owned textures differently. Callers also
//! provide realization for host-owned external layers.

use std::collections::{HashMap, HashSet};

use masonry_core::app::{ExternalVisualLayer, SceneVisualLayer, VisualLayerId, VisualLayerPlan};
use masonry_core::kurbo::{Rect, Size};
use masonry_imaging::vello::{
    TargetRenderer as VelloTargetRenderer, TextureTarget, new_target_renderer,
};
use masonry_imaging::{PreparedLayer, TextureRenderer, WindowSource};
use subduction_backend_wgpu::WgpuPresenter;
use subduction_core::backend::Presenter;
use subduction_core::layer::{ClipShape, LayerStore, SurfaceId};
use wgpu::StoreOp;

use crate::app_driver::{PresentVisualLayersResult, PresentationTarget};

type BoxError = Box<dyn std::error::Error + Send + Sync + 'static>;

/// The target texture and GPU context for rendering one external visual layer.
pub struct ExternalLayerTarget<'a> {
    /// Device used for rendering.
    pub device: &'a wgpu::Device,
    /// Queue used for uploads and submission.
    pub queue: &'a wgpu::Queue,
    /// Texture view owned by the subduction presenter for this layer.
    pub view: &'a wgpu::TextureView,
    /// The format used for layer textures in the presenter.
    pub format: wgpu::TextureFormat,
    /// Output size in physical pixels.
    pub output_size: winit::dpi::PhysicalSize<u32>,
    /// Window scale factor used to convert logical layer geometry into pixels.
    pub scale_factor: f64,
}

/// The target texture and GPU context for rendering one Masonry scene layer.
pub struct SceneLayerTarget<'a> {
    /// Device used for rendering.
    pub device: &'a wgpu::Device,
    /// Queue used for uploads and submission.
    pub queue: &'a wgpu::Queue,
    /// Texture view owned by the compositor for this layer.
    pub view: &'a wgpu::TextureView,
    /// Output size in physical pixels.
    pub output_size: winit::dpi::PhysicalSize<u32>,
    /// Window scale factor used to convert logical layer geometry into pixels.
    pub scale_factor: f64,
}

/// Render Masonry scene-backed visual layers into caller-provided textures.
pub trait SceneLayerRenderer {
    /// Render the given scene layer into `target.view`.
    fn render_scene_layer(
        &mut self,
        target: SceneLayerTarget<'_>,
        layer: SceneVisualLayer<'_>,
    ) -> Result<(), BoxError>;
}

/// Realize host-owned external layers into the textures allocated by the subduction presenter.
pub trait ExternalLayerRenderer {
    /// Synchronize the renderer with the current visual-layer plan before per-layer rendering.
    ///
    /// This hook is the right place to create or retire external-layer state keyed by layer id,
    /// and to request another redraw for continuously animated content.
    fn sync_external_layers(
        &mut self,
        _target: &PresentationTarget<'_>,
        _layers: &VisualLayerPlan,
    ) -> Result<ExternalLayerRenderResult, BoxError> {
        Ok(ExternalLayerRenderResult::default())
    }

    /// Render the given external visual layer into `target.view`.
    fn render_external_layer(
        &mut self,
        target: ExternalLayerTarget<'_>,
        layer: ExternalVisualLayer,
    ) -> Result<ExternalLayerRenderResult, BoxError>;
}

/// Stateful external-layer realization keyed by Masonry layer id.
///
/// This helper owns the common create/update/remove bookkeeping for host-managed external layers.
/// Integrations provide a renderer implementation that knows how to allocate any per-layer state
/// and how to render each layer into the texture supplied by [`SubductionPresenter`].
#[derive(Debug, Default)]
pub struct ManagedExternalLayers<R: ManagedExternalLayerRenderer> {
    renderer: R,
    layers: HashMap<VisualLayerId, R::LayerState>,
}

impl<R: ManagedExternalLayerRenderer> ManagedExternalLayers<R> {
    /// Create a new managed external-layer renderer around `renderer`.
    pub fn new(renderer: R) -> Self {
        Self {
            renderer,
            layers: HashMap::new(),
        }
    }

    /// Borrow the wrapped renderer mutably.
    pub fn renderer_mut(&mut self) -> &mut R {
        &mut self.renderer
    }

    /// Consume the manager and return the wrapped renderer.
    pub fn into_inner(self) -> R {
        self.renderer
    }
}

impl<R: ManagedExternalLayerRenderer> ExternalLayerRenderer for ManagedExternalLayers<R> {
    fn sync_external_layers(
        &mut self,
        target: &PresentationTarget<'_>,
        layers: &VisualLayerPlan,
    ) -> Result<ExternalLayerRenderResult, BoxError> {
        let mut result = self.renderer.begin_frame(target, layers)?;
        let mut driver = ManagedLayerDriver {
            renderer: &mut self.renderer,
            target,
        };
        sync_external_layer_state(
            &mut self.layers,
            layers
                .external_layers()
                .map(|(_, layer)| ExternalLayerSlot {
                    id: layer.id,
                    layer,
                }),
            &mut driver,
            &mut result,
        )?;

        Ok(result)
    }

    fn render_external_layer(
        &mut self,
        target: ExternalLayerTarget<'_>,
        layer: ExternalVisualLayer,
    ) -> Result<ExternalLayerRenderResult, BoxError> {
        let layer_id = layer.id;
        let state = self
            .layers
            .get_mut(&layer_id)
            .ok_or_else(|| std::io::Error::other("external layer state was not prepared"))?;
        self.renderer.render_layer(target, layer, state)
    }
}

/// Host-specific renderer for external layers with optional per-layer state.
pub trait ManagedExternalLayerRenderer {
    /// Per-layer state keyed by Masonry layer id.
    type LayerState;

    /// Synchronize any global frame state before per-layer updates and rendering.
    fn begin_frame(
        &mut self,
        _target: &PresentationTarget<'_>,
        _layers: &VisualLayerPlan,
    ) -> Result<ExternalLayerRenderResult, BoxError> {
        Ok(ExternalLayerRenderResult::default())
    }

    /// Create state for a newly visible external layer.
    fn create_layer(
        &mut self,
        target: &PresentationTarget<'_>,
        layer: ExternalVisualLayer,
    ) -> Result<Self::LayerState, BoxError>;

    /// Update state for an external layer that is visible this frame.
    fn update_layer(
        &mut self,
        _target: &PresentationTarget<'_>,
        _layer: ExternalVisualLayer,
        _state: &mut Self::LayerState,
    ) -> Result<ExternalLayerRenderResult, BoxError> {
        Ok(ExternalLayerRenderResult::default())
    }

    /// Drop state for an external layer that is no longer visible.
    fn destroy_layer(&mut self, _state: Self::LayerState) {}

    /// Render a single external layer into the texture allocated by [`SubductionPresenter`].
    fn render_layer(
        &mut self,
        target: ExternalLayerTarget<'_>,
        layer: ExternalVisualLayer,
        state: &mut Self::LayerState,
    ) -> Result<ExternalLayerRenderResult, BoxError>;
}

#[derive(Clone, Copy)]
struct ExternalLayerSlot<Id = VisualLayerId, Layer = ExternalVisualLayer> {
    id: Id,
    layer: Layer,
}

trait ExternalLayerStateDriver<Layer, State> {
    fn create_layer(&mut self, layer: &Layer) -> Result<State, BoxError>;
    fn update_layer(
        &mut self,
        layer: &Layer,
        state: &mut State,
    ) -> Result<ExternalLayerRenderResult, BoxError>;
    fn destroy_layer(&mut self, state: State);
}

struct ManagedLayerDriver<'a, 'b, R> {
    renderer: &'a mut R,
    target: &'a PresentationTarget<'b>,
}

impl<R: ManagedExternalLayerRenderer> ExternalLayerStateDriver<ExternalVisualLayer, R::LayerState>
    for ManagedLayerDriver<'_, '_, R>
{
    fn create_layer(&mut self, layer: &ExternalVisualLayer) -> Result<R::LayerState, BoxError> {
        self.renderer.create_layer(self.target, *layer)
    }

    fn update_layer(
        &mut self,
        layer: &ExternalVisualLayer,
        state: &mut R::LayerState,
    ) -> Result<ExternalLayerRenderResult, BoxError> {
        self.renderer.update_layer(self.target, *layer, state)
    }

    fn destroy_layer(&mut self, state: R::LayerState) {
        self.renderer.destroy_layer(state);
    }
}

fn sync_external_layer_state<Id: Copy + Eq + std::hash::Hash, Layer: Copy, State>(
    states: &mut HashMap<Id, State>,
    layers: impl IntoIterator<Item = ExternalLayerSlot<Id, Layer>>,
    driver: &mut impl ExternalLayerStateDriver<Layer, State>,
    result: &mut ExternalLayerRenderResult,
) -> Result<(), BoxError> {
    let mut live_ids = HashSet::new();

    for slot in layers {
        live_ids.insert(slot.id);

        if let Some(state) = states.get_mut(&slot.id) {
            result.combine(driver.update_layer(&slot.layer, state)?);
        } else {
            let mut state = driver.create_layer(&slot.layer)?;
            result.combine(driver.update_layer(&slot.layer, &mut state)?);
            states.insert(slot.id, state);
        }
    }

    let retired: Vec<_> = states
        .keys()
        .copied()
        .filter(|layer_id| !live_ids.contains(layer_id))
        .collect();
    for layer_id in retired {
        if let Some(state) = states.remove(&layer_id) {
            driver.destroy_layer(state);
        }
    }

    Ok(())
}

/// Result of synchronizing or rendering host-owned external layers for one frame.
#[derive(Clone, Copy, Debug, Default, PartialEq, Eq)]
pub struct ExternalLayerRenderResult {
    /// Whether the host wants another redraw immediately after this frame is presented.
    pub request_redraw: bool,
}

impl ExternalLayerRenderResult {
    /// Merge another per-frame result into this one.
    pub fn combine(&mut self, other: Self) {
        self.request_redraw |= other.request_redraw;
    }
}

/// Errors that can occur while presenting Masonry visual layers through `subduction`.
#[derive(Debug)]
pub enum PresentError {
    /// Rendering a Masonry scene layer failed.
    RenderScene(BoxError),
    /// Rendering a host-owned external layer failed.
    RenderExternal(BoxError),
}

impl core::fmt::Display for PresentError {
    fn fmt(&self, f: &mut core::fmt::Formatter<'_>) -> core::fmt::Result {
        match self {
            Self::RenderScene(err) => write!(f, "rendering Masonry scene layer failed: {err}"),
            Self::RenderExternal(err) => write!(f, "rendering external layer failed: {err}"),
        }
    }
}

impl std::error::Error for PresentError {}

/// Stateful `subduction` presenter for compositing Masonry visual layers into one output.
#[derive(Debug, Default)]
pub struct SubductionPresenter {
    state: Option<State>,
}

/// Vello-backed renderer for Masonry scene visual layers.
#[derive(Debug, Default)]
pub struct VelloSceneLayerRenderer {
    state: Option<VelloSceneRendererState>,
}

#[derive(Debug)]
struct State {
    device_key: usize,
    queue_key: usize,
    output_format: wgpu::TextureFormat,
    output_size: winit::dpi::PhysicalSize<u32>,
    presenter: WgpuPresenter,
}

#[derive(Debug)]
struct VelloSceneRendererState {
    device_key: usize,
    queue_key: usize,
    renderer: VelloTargetRenderer,
}

impl State {
    fn new(target: &PresentationTarget<'_>) -> Self {
        let presenter = WgpuPresenter::new(
            target.device.clone(),
            target.queue.clone(),
            target.format,
            (target.size.width, target.size.height),
            (target.size.width.max(1), target.size.height.max(1)),
        );
        Self {
            device_key: target.device as *const _ as usize,
            queue_key: target.queue as *const _ as usize,
            output_format: target.format,
            output_size: target.size,
            presenter,
        }
    }

    fn matches(&self, target: &PresentationTarget<'_>) -> bool {
        self.device_key == target.device as *const _ as usize
            && self.queue_key == target.queue as *const _ as usize
            && self.output_format == target.format
            && self.output_size == target.size
    }
}

impl VelloSceneRendererState {
    fn new(target: &SceneLayerTarget<'_>) -> Result<Self, masonry_imaging::vello::Error> {
        let renderer = new_target_renderer(target.device.clone(), target.queue.clone())?;
        Ok(Self {
            device_key: target.device as *const _ as usize,
            queue_key: target.queue as *const _ as usize,
            renderer,
        })
    }

    fn matches(&self, target: &SceneLayerTarget<'_>) -> bool {
        self.device_key == target.device as *const _ as usize
            && self.queue_key == target.queue as *const _ as usize
    }
}

impl SubductionPresenter {
    /// Create an empty presenter state.
    pub fn new() -> Self {
        Self::default()
    }

    /// Present the ordered Masonry visual layers into `target.view`.
    ///
    /// `scene_renderer` is responsible for turning Masonry scene layers into layer textures.
    /// `external_renderer` realizes host-owned external layers into the textures allocated by the
    /// presenter.
    pub fn present(
        &mut self,
        target: PresentationTarget<'_>,
        layers: &VisualLayerPlan,
        scene_renderer: &mut impl SceneLayerRenderer,
        external_renderer: &mut impl ExternalLayerRenderer,
    ) -> Result<PresentVisualLayersResult, PresentError> {
        if self
            .state
            .as_ref()
            .is_none_or(|state| !state.matches(&target))
        {
            self.state = Some(State::new(&target));
        }
        let state = self.state.as_mut().expect("presenter state should exist");
        let mut frame_result = external_renderer
            .sync_external_layers(&target, layers)
            .map_err(PresentError::RenderExternal)?;

        let mut store = LayerStore::new();
        let root = store.create_layer();
        let full_window = Size::new(f64::from(target.size.width), f64::from(target.size.height));

        let background = store.create_layer();
        store.add_child(root, background);
        store.set_content(background, Some(SurfaceId(0)));
        store.set_bounds(background, full_window);

        for (index, layer) in layers.layers.iter().enumerate() {
            let layer_id = store.create_layer();
            store.add_child(root, layer_id);
            store.set_content(layer_id, Some(surface_id_for_index(index)));
            store.set_bounds(layer_id, full_window);
            if let Some(clip) = layer.window_clip_bounds() {
                store.set_clip(
                    layer_id,
                    Some(ClipShape::Rect(scale_rect(clip, target.scale_factor))),
                );
            }
        }

        let changes = store.evaluate();
        state.presenter.apply(&store, &changes);

        if let Some(view) = state.presenter.texture_for_surface(SurfaceId(0)) {
            clear_texture_view(
                target.device,
                target.queue,
                view,
                to_wgpu_color(target.base_color),
            );
        }

        for (index, layer) in layers.layers.iter().enumerate() {
            let surface_id = surface_id_for_index(index);
            let Some(view) = state.presenter.texture_for_surface(surface_id) else {
                continue;
            };
            if let Some(scene_layer) = layer.as_scene() {
                scene_renderer
                    .render_scene_layer(
                        SceneLayerTarget {
                            device: target.device,
                            queue: target.queue,
                            view,
                            output_size: target.size,
                            scale_factor: target.scale_factor,
                        },
                        scene_layer,
                    )
                    .map_err(PresentError::RenderScene)?;
            } else if let Some(external_layer) = layer.as_external() {
                external_renderer
                    .render_external_layer(
                        ExternalLayerTarget {
                            device: target.device,
                            queue: target.queue,
                            view,
                            format: state.presenter.layer_format(),
                            output_size: target.size,
                            scale_factor: target.scale_factor,
                        },
                        external_layer,
                    )
                    .map(|result| frame_result.combine(result))
                    .map_err(PresentError::RenderExternal)?;
            }
        }

        let composite = state.presenter.composite(&store, target.view);
        target.queue.submit([composite]);
        Ok(PresentVisualLayersResult::Presented {
            request_redraw: frame_result.request_redraw,
        })
    }
}

impl VelloSceneLayerRenderer {
    /// Create an empty Vello scene-layer renderer state.
    pub fn new() -> Self {
        Self::default()
    }
}

impl SceneLayerRenderer for VelloSceneLayerRenderer {
    fn render_scene_layer(
        &mut self,
        target: SceneLayerTarget<'_>,
        layer: SceneVisualLayer<'_>,
    ) -> Result<(), BoxError> {
        if self
            .state
            .as_ref()
            .is_none_or(|state| !state.matches(&target))
        {
            self.state = Some(VelloSceneRendererState::new(&target).map_err(Box::new)?);
        }
        let state = self
            .state
            .as_mut()
            .expect("scene renderer state should exist");

        let prepared = [PreparedLayer::scene(
            layer.scene,
            layer.boundary,
            layer.bounds,
            layer.clip,
            layer.transform,
        )];
        let mut source = WindowSource::from_prepared_layers(
            target.output_size.width,
            target.output_size.height,
            target.scale_factor,
            masonry_core::peniko::Color::from_rgba8(0, 0, 0, 0),
            &prepared,
        );
        state
            .renderer
            .render_source_to_texture(
                &mut source,
                TextureTarget::new(
                    target.view,
                    target.output_size.width,
                    target.output_size.height,
                ),
            )
            .map_err(|err| Box::new(std::io::Error::other(format!("{err:?}"))) as BoxError)
    }
}

fn surface_id_for_index(index: usize) -> SurfaceId {
    let slot = u32::try_from(index + 1).expect("layer count should fit in u32");
    SurfaceId(slot)
}

fn scale_rect(rect: Rect, scale_factor: f64) -> Rect {
    Rect::new(
        rect.x0 * scale_factor,
        rect.y0 * scale_factor,
        rect.x1 * scale_factor,
        rect.y1 * scale_factor,
    )
}

fn to_wgpu_color(color: masonry_core::peniko::Color) -> wgpu::Color {
    let rgba = color.to_rgba8();
    wgpu::Color {
        r: f64::from(rgba.r) / 255.0,
        g: f64::from(rgba.g) / 255.0,
        b: f64::from(rgba.b) / 255.0,
        a: f64::from(rgba.a) / 255.0,
    }
}

fn clear_texture_view(
    device: &wgpu::Device,
    queue: &wgpu::Queue,
    view: &wgpu::TextureView,
    color: wgpu::Color,
) {
    let mut encoder = device.create_command_encoder(&wgpu::CommandEncoderDescriptor {
        label: Some("subduction clear encoder"),
    });
    {
        let _pass = encoder.begin_render_pass(&wgpu::RenderPassDescriptor {
            label: Some("subduction clear pass"),
            color_attachments: &[Some(wgpu::RenderPassColorAttachment {
                view,
                resolve_target: None,
                depth_slice: None,
                ops: wgpu::Operations {
                    load: wgpu::LoadOp::Clear(color),
                    store: StoreOp::Store,
                },
            })],
            depth_stencil_attachment: None,
            timestamp_writes: None,
            occlusion_query_set: None,
            multiview_mask: None,
        });
    }
    queue.submit([encoder.finish()]);
}

#[cfg(test)]
mod tests {
    use super::{
        ExternalLayerRenderResult, ExternalLayerSlot, ExternalLayerStateDriver,
        sync_external_layer_state,
    };
    use std::collections::HashMap;

    #[derive(Clone, Copy)]
    struct FakeLayer {
        id: u64,
        request_redraw: bool,
    }

    #[derive(Default)]
    struct FakeDriver {
        created: Vec<u64>,
        updated: Vec<(u64, u64)>,
        destroyed: Vec<u64>,
    }

    impl ExternalLayerStateDriver<FakeLayer, u64> for FakeDriver {
        fn create_layer(&mut self, layer: &FakeLayer) -> Result<u64, super::BoxError> {
            self.created.push(layer.id);
            Ok(layer.id)
        }

        fn update_layer(
            &mut self,
            layer: &FakeLayer,
            state: &mut u64,
        ) -> Result<ExternalLayerRenderResult, super::BoxError> {
            self.updated.push((layer.id, *state));
            Ok(ExternalLayerRenderResult {
                request_redraw: layer.request_redraw,
            })
        }

        fn destroy_layer(&mut self, state: u64) {
            self.destroyed.push(state);
        }
    }

    #[test]
    fn sync_external_layer_state_creates_updates_and_retires_layers() {
        let first_layer = FakeLayer {
            id: 11,
            request_redraw: false,
        };
        let second_layer = FakeLayer {
            id: 29,
            request_redraw: false,
        };

        let mut states = HashMap::new();
        let mut driver = FakeDriver::default();
        let mut frame_result = ExternalLayerRenderResult::default();

        sync_external_layer_state(
            &mut states,
            [ExternalLayerSlot {
                id: first_layer.id,
                layer: first_layer,
            }],
            &mut driver,
            &mut frame_result,
        )
        .unwrap();

        assert_eq!(driver.created, vec![first_layer.id]);
        assert_eq!(driver.updated, vec![(first_layer.id, first_layer.id)]);
        assert!(driver.destroyed.is_empty());
        assert_eq!(states.len(), 1);

        driver.created.clear();
        driver.updated.clear();
        sync_external_layer_state(
            &mut states,
            [
                ExternalLayerSlot {
                    id: first_layer.id,
                    layer: first_layer,
                },
                ExternalLayerSlot {
                    id: second_layer.id,
                    layer: second_layer,
                },
            ],
            &mut driver,
            &mut frame_result,
        )
        .unwrap();

        assert_eq!(driver.created, vec![second_layer.id]);
        assert_eq!(
            driver.updated,
            vec![
                (first_layer.id, first_layer.id),
                (second_layer.id, second_layer.id)
            ]
        );
        assert!(driver.destroyed.is_empty());
        assert_eq!(states.len(), 2);

        driver.updated.clear();
        sync_external_layer_state(
            &mut states,
            [ExternalLayerSlot {
                id: second_layer.id,
                layer: second_layer,
            }],
            &mut driver,
            &mut frame_result,
        )
        .unwrap();

        assert_eq!(driver.updated, vec![(second_layer.id, second_layer.id)]);
        assert_eq!(driver.destroyed, vec![first_layer.id]);
        assert_eq!(states.len(), 1);
    }

    #[test]
    fn sync_external_layer_state_combines_redraw_requests() {
        let first_layer = FakeLayer {
            id: 3,
            request_redraw: false,
        };
        let second_layer = FakeLayer {
            id: 7,
            request_redraw: true,
        };

        let mut states = HashMap::new();
        let mut driver = FakeDriver::default();
        let mut frame_result = ExternalLayerRenderResult::default();

        sync_external_layer_state(
            &mut states,
            [
                ExternalLayerSlot {
                    id: first_layer.id,
                    layer: first_layer,
                },
                ExternalLayerSlot {
                    id: second_layer.id,
                    layer: second_layer,
                },
            ],
            &mut driver,
            &mut frame_result,
        )
        .unwrap();

        assert!(frame_result.request_redraw);
    }
}
