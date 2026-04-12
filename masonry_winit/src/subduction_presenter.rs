// Copyright 2026 the Xilem Authors
// SPDX-License-Identifier: Apache-2.0

//! Reusable `subduction`-backed presentation for Masonry visual layers.
//!
//! `SubductionPresenter` owns the compositor state and the default Masonry scene-layer renderer.
//! Hosts only provide texture contents for external visual layers.

use std::collections::{HashMap, HashSet};

use masonry_core::app::{VisualLayer, VisualLayerId, VisualLayerKind, VisualLayerPlan};
use masonry_core::kurbo::Size;
use masonry_imaging::vello::{TargetRenderer as VelloTargetRenderer, new_target_renderer};
use subduction_backend_wgpu::WgpuPresenter;
use subduction_core::backend::Presenter;
use subduction_core::layer::{ClipShape, LayerId, LayerStore, SurfaceId};
use wgpu::StoreOp;

use crate::app_driver::PresentationTarget;

type BoxError = Box<dyn std::error::Error + Send + Sync + 'static>;

/// The GPU target for rendering one visual layer texture.
pub struct LayerTextureTarget<'a> {
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

/// Stateful `subduction` presenter for compositing Masonry visual layers into one output.
#[derive(Debug, Default)]
pub struct SubductionPresenter {
    compositor: Option<CompositorState>,
    scene_renderer: Option<VelloSceneRendererState>,
}

#[derive(Debug)]
struct CompositorState {
    device_key: usize,
    queue_key: usize,
    output_format: wgpu::TextureFormat,
    output_size: winit::dpi::PhysicalSize<u32>,
    presenter: WgpuPresenter,
    layers: LayerSyncState,
}

#[derive(Debug)]
struct VelloSceneRendererState {
    device_key: usize,
    queue_key: usize,
    renderer: VelloTargetRenderer,
}

#[derive(Clone, Copy, Debug)]
struct PresentedLayer {
    layer_id: LayerId,
    surface_id: SurfaceId,
}

#[derive(Debug)]
struct LayerSyncState {
    store: LayerStore,
    root: LayerId,
    layers: HashMap<VisualLayerId, PresentedLayer>,
    ordered_ids: Vec<VisualLayerId>,
    next_surface_id: u32,
}

impl CompositorState {
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
            layers: LayerSyncState::new(),
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
    fn new(target: &LayerTextureTarget<'_>) -> Result<Self, masonry_imaging::vello::Error> {
        let renderer = new_target_renderer(target.device.clone(), target.queue.clone())?;
        Ok(Self {
            device_key: target.device as *const _ as usize,
            queue_key: target.queue as *const _ as usize,
            renderer,
        })
    }

    fn matches(&self, target: &LayerTextureTarget<'_>) -> bool {
        self.device_key == target.device as *const _ as usize
            && self.queue_key == target.queue as *const _ as usize
    }
}

impl LayerSyncState {
    fn new() -> Self {
        let mut store = LayerStore::new();
        let root = store.create_layer();

        Self {
            store,
            root,
            layers: HashMap::new(),
            ordered_ids: Vec::new(),
            next_surface_id: 1,
        }
    }

    fn sync(
        &mut self,
        layers: &VisualLayerPlan,
        output_size: winit::dpi::PhysicalSize<u32>,
        scale_factor: f64,
    ) -> &LayerStore {
        let desired_ids: Vec<_> = layers.layers.iter().map(|layer| layer.id()).collect();
        let live_ids: HashSet<_> = desired_ids.iter().copied().collect();
        let stale_ids: Vec<_> = self
            .layers
            .keys()
            .copied()
            .filter(|layer_id| !live_ids.contains(layer_id))
            .collect();
        for layer_id in stale_ids {
            let presented = self
                .layers
                .remove(&layer_id)
                .expect("stale layer should have presentation state");
            self.store.destroy_layer(presented.layer_id);
        }

        for layer in &layers.layers {
            let presented = self.layers.entry(layer.id()).or_insert_with(|| {
                let layer_id = self.store.create_layer();
                let surface_id = SurfaceId(self.next_surface_id);
                self.next_surface_id += 1;
                PresentedLayer {
                    layer_id,
                    surface_id,
                }
            });
            Self::sync_layer_properties(
                &mut self.store,
                *presented,
                layer,
                output_size,
                scale_factor,
            );
        }

        if self.ordered_ids != desired_ids {
            for layer_id in &desired_ids {
                let presented = self
                    .layers
                    .get(layer_id)
                    .copied()
                    .expect("ordered visual layer should have presentation state");
                self.store.reparent(presented.layer_id, self.root);
            }
            self.ordered_ids = desired_ids;
        }

        &self.store
    }

    fn surface_id_for_layer(&self, id: VisualLayerId) -> Option<SurfaceId> {
        self.layers.get(&id).map(|layer| layer.surface_id)
    }

    fn sync_layer_properties(
        store: &mut LayerStore,
        presented: PresentedLayer,
        layer: &VisualLayer,
        output_size: winit::dpi::PhysicalSize<u32>,
        scale_factor: f64,
    ) {
        let full_window = Size::new(f64::from(output_size.width), f64::from(output_size.height));
        if store.content(presented.layer_id) != Some(presented.surface_id) {
            store.set_content(presented.layer_id, Some(presented.surface_id));
        }
        if store.bounds(presented.layer_id) != full_window {
            store.set_bounds(presented.layer_id, full_window);
        }

        let clip = layer
            .window_clip_bounds()
            .map(|clip| ClipShape::Rect(clip.scale_from_origin(scale_factor)));
        if store.clip(presented.layer_id) != clip {
            store.set_clip(presented.layer_id, clip);
        }
    }
}

impl SubductionPresenter {
    /// Create an empty presenter state.
    pub fn new() -> Self {
        Self::default()
    }

    /// Present the ordered Masonry visual layers into `target.view`.
    ///
    /// Masonry scene layers are rendered internally through `masonry_imaging`'s Vello target
    /// renderer. `external_renderer` supplies the texture contents for host-owned external
    /// layers.
    pub fn present(
        &mut self,
        target: PresentationTarget<'_>,
        layers: &VisualLayerPlan,
        request_redraw: bool,
        mut render_external_layer: impl FnMut(
            LayerTextureTarget<'_>,
            &VisualLayer,
        ) -> Result<(), BoxError>,
    ) -> Result<bool, BoxError> {
        if self
            .compositor
            .as_ref()
            .is_none_or(|state| !state.matches(&target))
        {
            self.compositor = Some(CompositorState::new(&target));
        }

        let layer_format = {
            let compositor = self
                .compositor
                .as_mut()
                .expect("compositor state should exist");
            compositor
                .layers
                .sync(layers, target.size, target.scale_factor);
            let changes = compositor.layers.store.evaluate();
            compositor
                .presenter
                .apply(&compositor.layers.store, &changes);
            compositor.presenter.layer_format()
        };

        let rgba = target.base_color.to_rgba8();
        // TODO: This clear belongs in `subduction_backend_wgpu`, alongside final composition.
        // Masonry should describe visual layers, not own compositor output initialization policy.
        clear_texture_view(
            target.device,
            target.queue,
            target.view,
            wgpu::Color {
                r: f64::from(rgba.r) / 255.0,
                g: f64::from(rgba.g) / 255.0,
                b: f64::from(rgba.b) / 255.0,
                a: f64::from(rgba.a) / 255.0,
            },
        );

        for layer in &layers.layers {
            let surface_id = self
                .compositor
                .as_ref()
                .expect("compositor state should exist")
                .layers
                .surface_id_for_layer(layer.id())
                .expect("visual layer should have a stable surface id");
            let Some(view) = self
                .compositor
                .as_ref()
                .expect("compositor state should exist")
                .presenter
                .texture_for_surface(surface_id)
                .cloned()
            else {
                continue;
            };
            let layer_target = LayerTextureTarget {
                device: target.device,
                queue: target.queue,
                view: &view,
                format: layer_format,
                output_size: target.size,
                scale_factor: target.scale_factor,
            };

            if matches!(layer.kind, VisualLayerKind::Scene(_)) {
                self.render_scene_layer(layer_target, layer)?;
            } else if matches!(layer.kind, VisualLayerKind::External) {
                render_external_layer(layer_target, layer)?;
            }
        }

        let composite = {
            let compositor = self
                .compositor
                .as_mut()
                .expect("compositor state should exist");
            let store = &compositor.layers.store;
            compositor.presenter.composite(store, target.view)
        };
        target.queue.submit([composite]);
        Ok(request_redraw)
    }

    fn render_scene_layer(
        &mut self,
        target: LayerTextureTarget<'_>,
        layer: &VisualLayer,
    ) -> Result<(), BoxError> {
        if self
            .scene_renderer
            .as_ref()
            .is_none_or(|state| !state.matches(&target))
        {
            self.scene_renderer = Some(VelloSceneRendererState::new(&target).map_err(Box::new)?);
        }
        let state = self
            .scene_renderer
            .as_mut()
            .expect("scene renderer state should exist");
        // TODO: `subduction_backend_wgpu` should expose the presenter-owned `wgpu::Texture`, not
        // just its `TextureView`, so Masonry can render scene layers through the backend-neutral
        // imaging path instead of keeping this Vello-specific direct-to-view fallback.
        masonry_imaging::vello::render_scene_layer_to_texture(
            &mut state.renderer,
            target.view,
            target.output_size.width,
            target.output_size.height,
            target.scale_factor,
            layer,
        )
        .map_err(Box::new)
        .map_err(|err| err as BoxError)
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
    use super::LayerSyncState;
    use masonry_core::accesskit::{Node, Role};
    use masonry_core::app::{VisualLayer, VisualLayerBoundary, VisualLayerPlan};
    use masonry_core::core::{
        AccessCtx, ChildrenIds, LayoutCtx, MeasureCtx, NoAction, PaintCtx, PropertiesRef,
        RegisterCtx, Widget, WidgetPod,
    };
    use masonry_core::imaging::Painter;
    use masonry_core::kurbo::{Axis, Rect, Size};
    use masonry_core::layout::LenReq;
    use winit::dpi::PhysicalSize;

    struct TestWidget;

    impl Widget for TestWidget {
        type Action = NoAction;

        fn register_children(&mut self, _ctx: &mut RegisterCtx<'_>) {}

        fn measure(
            &mut self,
            _ctx: &mut MeasureCtx<'_>,
            _props: &PropertiesRef<'_>,
            _axis: Axis,
            _len_req: LenReq,
            _cross_length: Option<f64>,
        ) -> f64 {
            10.0
        }

        fn layout(&mut self, _ctx: &mut LayoutCtx<'_>, _props: &PropertiesRef<'_>, _size: Size) {}

        fn paint(
            &mut self,
            _ctx: &mut PaintCtx<'_>,
            _props: &PropertiesRef<'_>,
            _painter: &mut Painter<'_>,
        ) {
        }

        fn accessibility_role(&self) -> Role {
            Role::GenericContainer
        }

        fn accessibility(
            &mut self,
            _ctx: &mut AccessCtx<'_>,
            _props: &PropertiesRef<'_>,
            _node: &mut Node,
        ) {
        }

        fn children_ids(&self) -> ChildrenIds {
            ChildrenIds::new()
        }
    }

    fn external_plan(root_id: masonry_core::core::WidgetId, layer_count: usize) -> VisualLayerPlan {
        let layers = (0..layer_count)
            .map(|index| {
                let x = 10.0 + (index as f64) * 30.0;
                VisualLayer::external(
                    VisualLayerBoundary::WidgetBoundary,
                    Rect::new(0.0, 0.0, 20.0, 20.0),
                    Some(Rect::new(0.0, 0.0, 20.0, 20.0)),
                    masonry_core::kurbo::Affine::translate((x, 5.0)),
                    root_id,
                )
            })
            .collect();
        VisualLayerPlan::new(layers)
    }

    #[test]
    fn sync_reuses_layer_ids_for_unchanged_plan() {
        let output_size = PhysicalSize::new(200, 100);
        let mut sync = LayerSyncState::new();
        let root_id = WidgetPod::new(TestWidget).id();
        let plan = external_plan(root_id, 2);

        sync.sync(&plan, output_size, 2.0);
        let first = sync.store.evaluate();
        assert_eq!(first.added.len(), 3);

        sync.sync(&plan, output_size, 2.0);
        let second = sync.store.evaluate();
        assert!(second.added.is_empty());
        assert!(second.removed.is_empty());
        assert!(second.content.is_empty());
        assert!(second.bounds.is_empty());
        assert!(second.clips.is_empty());
        assert!(!second.topology_changed);
    }

    #[test]
    fn sync_removes_stale_visual_layers_without_readding_the_rest() {
        let output_size = PhysicalSize::new(200, 100);
        let mut sync = LayerSyncState::new();
        let root_id = WidgetPod::new(TestWidget).id();

        sync.sync(&external_plan(root_id, 2), output_size, 1.0);
        let _ = sync.store.evaluate();

        sync.sync(&external_plan(root_id, 1), output_size, 1.0);
        let changes = sync.store.evaluate();
        assert!(changes.added.is_empty());
        assert_eq!(changes.removed.len(), 1);
    }
}
