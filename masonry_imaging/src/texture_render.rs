// Copyright 2026 the Xilem Authors
// SPDX-License-Identifier: Apache-2.0

//! Host-neutral texture rendering helpers.
//!
//! This module owns backend state for rendering Masonry visual layers into a caller-provided WGPU
//! texture target. It does not own window surfaces or presentation.

use wgpu;

use masonry_core::app::VisualLayerPlan;
use peniko::Color;

/// GPU target that Masonry content should be rendered into.
#[derive(Clone, Copy, Debug)]
pub struct RenderTarget<'a> {
    /// Adapter used to create the device.
    pub adapter: &'a wgpu::Adapter,
    /// Device used for rendering commands.
    pub device: &'a wgpu::Device,
    /// Queue used for submitting rendering commands and uploads.
    pub queue: &'a wgpu::Queue,
    /// Texture backing the render target.
    pub texture: &'a wgpu::Texture,
    /// View of the render target texture.
    pub view: &'a wgpu::TextureView,
}

/// Masonry paint output prepared for texture rendering.
#[derive(Clone, Copy)]
pub struct RenderInput<'a> {
    /// Output width in physical pixels.
    pub width: u32,
    /// Output height in physical pixels.
    pub height: u32,
    /// Output scale factor.
    pub scale_factor: f64,
    /// Background color to paint before replaying Masonry scene layers.
    pub background_color: Color,
    /// Ordered visual layers to render.
    pub visual_layers: &'a VisualLayerPlan,
}

impl core::fmt::Debug for RenderInput<'_> {
    fn fmt(&self, f: &mut core::fmt::Formatter<'_>) -> core::fmt::Result {
        f.debug_struct("RenderInput")
            .field("width", &self.width)
            .field("height", &self.height)
            .field("scale_factor", &self.scale_factor)
            .field("background_color", &self.background_color)
            .field("visual_layer_count", &self.visual_layers.layers.len())
            .finish()
    }
}

impl<'a> RenderInput<'a> {
    /// Create texture-render input directly from Masonry visual layers.
    pub fn new(
        width: u32,
        height: u32,
        scale_factor: f64,
        background_color: Color,
        visual_layers: &'a VisualLayerPlan,
    ) -> Self {
        Self {
            width,
            height,
            scale_factor,
            background_color,
            visual_layers,
        }
    }
}

/// Errors that can occur while rendering Masonry content into a target texture.
#[derive(Debug)]
pub struct Error(imp::Error);

impl core::fmt::Display for Error {
    fn fmt(&self, f: &mut core::fmt::Formatter<'_>) -> core::fmt::Result {
        self.0.fmt(f)
    }
}

impl std::error::Error for Error {}

/// Backend-selected texture renderer for Masonry paint output.
#[derive(Debug)]
pub struct Renderer(imp::Renderer);

impl Renderer {
    /// Stable backend name for diagnostics.
    pub const BACKEND_NAME: &str = imp::Renderer::BACKEND_NAME;

    /// Create an empty renderer state.
    pub fn new() -> Self {
        Self(imp::Renderer::new())
    }

    /// Render the given Masonry paint output into the provided target texture.
    pub fn render_to_texture(
        &mut self,
        target: RenderTarget<'_>,
        input: RenderInput<'_>,
    ) -> Result<(), Error> {
        self.0.render_to_texture(target, input).map_err(Error)
    }
}

impl Default for Renderer {
    fn default() -> Self {
        Self::new()
    }
}

#[cfg(all(
    not(feature = "imaging_vello"),
    any(
        all(feature = "imaging_skia", not(target_arch = "wasm32")),
        feature = "imaging_vello_hybrid"
    )
))]
mod non_vello {
    use imaging::render::TextureRenderer;

    #[derive(Debug)]
    pub(super) struct CachedRenderer<R, K> {
        key: Option<K>,
        inner: Option<R>,
    }

    impl<R, K> CachedRenderer<R, K>
    where
        K: Copy + Eq,
    {
        pub(super) fn new() -> Self {
            Self {
                key: None,
                inner: None,
            }
        }

        pub(super) fn get_or_try_init<E>(
            &mut self,
            key: K,
            create: impl FnOnce() -> Result<R, E>,
        ) -> Result<&mut R, E> {
            if self.key != Some(key) {
                self.inner = Some(create()?);
                self.key = Some(key);
            }

            Ok(self
                .inner
                .as_mut()
                .expect("cached renderer should be initialized"))
        }
    }

    pub(super) fn render_window_source_to_texture<R>(
        renderer: &mut R,
        input: super::RenderInput<'_>,
        target: R::TextureTarget<'_>,
    ) -> Result<(), R::Error>
    where
        R: TextureRenderer,
    {
        let mut source = crate::WindowSource::from_visual_layers(
            input.width,
            input.height,
            input.scale_factor,
            input.background_color,
            input.visual_layers,
        );
        renderer.render_source_to_texture(&mut source, target)
    }
}

#[cfg(not(any(
    feature = "imaging_vello",
    feature = "imaging_vello_hybrid",
    all(feature = "imaging_skia", not(target_arch = "wasm32"))
)))]
mod imp {
    #[derive(Debug)]
    pub(super) enum Error {}

    impl core::fmt::Display for Error {
        fn fmt(&self, _: &mut core::fmt::Formatter<'_>) -> core::fmt::Result {
            match *self {}
        }
    }

    impl std::error::Error for Error {}

    #[derive(Debug)]
    pub(super) struct Renderer;

    impl Renderer {
        pub(super) const BACKEND_NAME: &str = "";

        pub(super) fn new() -> Self {
            Self
        }

        pub(super) fn render_to_texture(
            &mut self,
            _: super::RenderTarget<'_>,
            _: super::RenderInput<'_>,
        ) -> Result<(), Error> {
            unreachable!("a renderer backend feature is required")
        }
    }
}

#[cfg(all(
    not(feature = "imaging_vello"),
    feature = "imaging_skia",
    not(target_arch = "wasm32")
))]
mod imp {
    use super::non_vello::{CachedRenderer, render_window_source_to_texture};
    use crate::skia::{TargetRenderer, TextureTarget, new_target_renderer};

    use super::{RenderInput, RenderTarget};

    /// Errors that can occur while rendering Masonry content with Skia.
    #[derive(Debug)]
    pub(super) enum Error {
        /// Creating the Skia renderer failed.
        CreateRenderer(crate::skia::Error),
        /// Rendering the Masonry source failed.
        Render(imaging_skia::Error),
    }

    impl core::fmt::Display for Error {
        fn fmt(&self, f: &mut core::fmt::Formatter<'_>) -> core::fmt::Result {
            match self {
                Self::CreateRenderer(err) => write!(f, "creating Skia renderer failed: {err}"),
                Self::Render(err) => write!(f, "rendering with Skia failed: {err:?}"),
            }
        }
    }

    impl std::error::Error for Error {}

    /// Runtime renderer state for the Skia backend.
    #[derive(Debug)]
    pub(super) struct Renderer {
        inner: CachedRenderer<TargetRenderer, (usize, usize, usize)>,
    }

    impl Renderer {
        /// Stable backend name for diagnostics.
        pub(super) const BACKEND_NAME: &str = crate::skia::BACKEND_NAME;

        /// Create an empty renderer state.
        pub(super) fn new() -> Self {
            Self {
                inner: CachedRenderer::new(),
            }
        }

        /// Render the given Masonry paint output into the provided target texture.
        pub(super) fn render_to_texture(
            &mut self,
            target: RenderTarget<'_>,
            input: RenderInput<'_>,
        ) -> Result<(), Error> {
            let renderer = self.inner.get_or_try_init(
                (
                    target.adapter as *const _ as usize,
                    target.device as *const _ as usize,
                    target.queue as *const _ as usize,
                ),
                || {
                    new_target_renderer(
                        target.adapter.clone(),
                        target.device.clone(),
                        target.queue.clone(),
                    )
                    .map_err(Error::CreateRenderer)
                },
            )?;

            render_window_source_to_texture(renderer, input, TextureTarget::new(target.texture))
                .map_err(Error::Render)
        }
    }
}

#[cfg(feature = "imaging_vello")]
impl Renderer {
    /// Set a persistent image override for the Vello backend.
    pub fn set_image_override(&mut self, image: peniko::ImageData, texture: wgpu::Texture) {
        self.0.set_image_override(image, texture);
    }

    /// Clear a previously-set Vello image override.
    pub fn clear_image_override(&mut self, image: &peniko::ImageData) {
        self.0.clear_image_override(image);
    }
}

#[cfg(feature = "imaging_vello")]
mod imp {
    use std::collections::HashMap;

    use vello::{AaConfig, AaSupport, RenderParams, Renderer as VelloRenderer, RendererOptions};

    use crate::vello::build_scene_from_source;

    use super::{RenderInput, RenderTarget};

    /// Errors that can occur while rendering Masonry content with Vello.
    #[derive(Debug)]
    pub(super) enum Error {
        /// Building the native Vello scene failed.
        BuildScene(crate::vello::Error),
        /// Creating the Vello renderer failed.
        CreateRenderer(vello::Error),
        /// Rendering the scene to the texture failed.
        Render(vello::Error),
    }

    impl core::fmt::Display for Error {
        fn fmt(&self, f: &mut core::fmt::Formatter<'_>) -> core::fmt::Result {
            match self {
                Self::BuildScene(err) => write!(f, "building Vello scene failed: {err}"),
                Self::CreateRenderer(err) => write!(f, "creating Vello renderer failed: {err}"),
                Self::Render(err) => write!(f, "rendering with Vello failed: {err}"),
            }
        }
    }

    impl std::error::Error for Error {}

    #[derive(Debug)]
    struct ImageOverrideState {
        image: peniko::ImageData,
        texture: wgpu::Texture,
        applied: bool,
        prev: Option<wgpu::TexelCopyTextureInfoBase<wgpu::Texture>>,
    }

    /// Runtime renderer state for the Vello backend.
    pub(super) struct Renderer {
        inner: Option<VelloRenderer>,
        image_overrides: HashMap<u64, ImageOverrideState>,
    }

    impl core::fmt::Debug for Renderer {
        fn fmt(&self, f: &mut core::fmt::Formatter<'_>) -> core::fmt::Result {
            f.debug_struct("Renderer")
                .field("inner", &self.inner.as_ref().map(|_| "(VelloRenderer)"))
                .field("image_overrides", &self.image_overrides)
                .finish()
        }
    }

    impl Renderer {
        /// Stable backend name for diagnostics.
        pub(super) const BACKEND_NAME: &str = crate::vello::BACKEND_NAME;

        /// Create an empty renderer state.
        pub(super) fn new() -> Self {
            Self {
                inner: None,
                image_overrides: HashMap::new(),
            }
        }

        /// Render the given Masonry paint output into the provided target texture.
        pub(super) fn render_to_texture(
            &mut self,
            target: RenderTarget<'_>,
            input: RenderInput<'_>,
        ) -> Result<(), Error> {
            let width = input.width;
            let height = input.height;
            let mut source = crate::WindowSource::from_visual_layers(
                input.width,
                input.height,
                input.scale_factor,
                input.background_color,
                input.visual_layers,
            );
            let scene =
                build_scene_from_source(&mut source, width, height).map_err(Error::BuildScene)?;

            if self.inner.is_none() {
                let renderer_options = RendererOptions {
                    antialiasing_support: AaSupport::area_only(),
                    ..Default::default()
                };
                self.inner = Some(
                    VelloRenderer::new(target.device, renderer_options)
                        .map_err(Error::CreateRenderer)?,
                );
            }
            let renderer = self.inner.as_mut().unwrap();

            for state in self.image_overrides.values_mut() {
                if state.applied {
                    continue;
                }
                state.prev = renderer.override_image(
                    &state.image,
                    Some(wgpu::TexelCopyTextureInfoBase {
                        texture: state.texture.clone(),
                        mip_level: 0,
                        origin: wgpu::Origin3d::ZERO,
                        aspect: wgpu::TextureAspect::All,
                    }),
                );
                state.applied = true;
            }

            let render_params = RenderParams {
                // WindowSource already paints the background into the scene, so keep
                // Vello's target clear transparent here instead of applying the base color twice.
                base_color: peniko::Color::from_rgba8(0, 0, 0, 0),
                width,
                height,
                antialiasing_method: AaConfig::Area,
            };
            renderer
                .render_to_texture(
                    target.device,
                    target.queue,
                    &scene,
                    target.view,
                    &render_params,
                )
                .map_err(Error::Render)
        }

        /// Set a persistent image override for the Vello backend.
        pub(super) fn set_image_override(
            &mut self,
            image: peniko::ImageData,
            texture: wgpu::Texture,
        ) {
            let image_id = image.data.id();

            if let Some(existing) = self.image_overrides.get_mut(&image_id) {
                existing.texture = texture;
                if existing.applied {
                    if let Some(renderer) = &mut self.inner {
                        renderer.override_image(
                            &existing.image,
                            Some(wgpu::TexelCopyTextureInfoBase {
                                texture: existing.texture.clone(),
                                mip_level: 0,
                                origin: wgpu::Origin3d::ZERO,
                                aspect: wgpu::TextureAspect::All,
                            }),
                        );
                    } else {
                        existing.applied = false;
                    }
                }
                return;
            }

            let mut state = ImageOverrideState {
                image,
                texture,
                applied: false,
                prev: None,
            };

            if let Some(renderer) = &mut self.inner {
                state.prev = renderer.override_image(
                    &state.image,
                    Some(wgpu::TexelCopyTextureInfoBase {
                        texture: state.texture.clone(),
                        mip_level: 0,
                        origin: wgpu::Origin3d::ZERO,
                        aspect: wgpu::TextureAspect::All,
                    }),
                );
                state.applied = true;
            }

            self.image_overrides.insert(image_id, state);
        }

        /// Clear a previously-set Vello image override.
        pub(super) fn clear_image_override(&mut self, image: &peniko::ImageData) {
            let image_id = image.data.id();
            let Some(state) = self.image_overrides.remove(&image_id) else {
                return;
            };
            if state.applied
                && let Some(renderer) = &mut self.inner
            {
                renderer.override_image(&state.image, state.prev);
            }
        }
    }
}

#[cfg(all(
    not(feature = "imaging_vello"),
    not(all(feature = "imaging_skia", not(target_arch = "wasm32"))),
    feature = "imaging_vello_hybrid"
))]
mod imp {
    use super::non_vello::{CachedRenderer, render_window_source_to_texture};
    use crate::vello_hybrid::{TargetRenderer, TextureTarget, new_target_renderer};

    use super::{RenderInput, RenderTarget};

    /// Errors that can occur while rendering Masonry content with Vello Hybrid.
    #[derive(Debug)]
    pub(super) enum Error {
        /// Rendering the Masonry source failed.
        Render(imaging_vello_hybrid::Error),
    }

    impl core::fmt::Display for Error {
        fn fmt(&self, f: &mut core::fmt::Formatter<'_>) -> core::fmt::Result {
            match self {
                Self::Render(err) => write!(f, "rendering with Vello Hybrid failed: {err:?}"),
            }
        }
    }

    impl std::error::Error for Error {}

    /// Runtime renderer state for the Vello Hybrid backend.
    #[derive(Debug)]
    pub(super) struct Renderer {
        inner: CachedRenderer<TargetRenderer, (usize, usize)>,
    }

    impl Renderer {
        /// Stable backend name for diagnostics.
        pub(super) const BACKEND_NAME: &str = crate::vello_hybrid::BACKEND_NAME;

        /// Create an empty renderer state.
        pub(super) fn new() -> Self {
            Self {
                inner: CachedRenderer::new(),
            }
        }

        /// Render the given Masonry paint output into the provided target texture.
        pub(super) fn render_to_texture(
            &mut self,
            target: RenderTarget<'_>,
            input: RenderInput<'_>,
        ) -> Result<(), Error> {
            let width = input.width;
            let height = input.height;
            let renderer = self.inner.get_or_try_init(
                (
                    target.device as *const _ as usize,
                    target.queue as *const _ as usize,
                ),
                || {
                    Ok(new_target_renderer(
                        target.device.clone(),
                        target.queue.clone(),
                    ))
                },
            )?;

            render_window_source_to_texture(
                renderer,
                input,
                TextureTarget::new(target.view, width, height),
            )
            .map_err(Error::Render)
        }
    }
}
