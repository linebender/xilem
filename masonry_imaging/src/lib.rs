// Copyright 2026 the Xilem Authors
// SPDX-License-Identifier: Apache-2.0

#![doc(
    html_logo_url = "https://raw.githubusercontent.com/linebender/xilem/main/docs/assets/masonry-logo.svg"
)]
//! Imaging helpers owned by Masonry.
//!
//! `masonry_imaging` owns the bridge between Masonry paint output and concrete imaging backends.
//! In this first slice that means:
//!
//! - preparing flattened Masonry frame content from base content plus overlays
//! - exposing backend-specific renderer modules for `imaging_vello`,
//!   `imaging_vello_hybrid`, `imaging_vello_cpu`, and `imaging_skia`
//! - exposing host-neutral texture rendering helpers for writing into caller-provided WGPU
//!   targets
//!
//! This crate does not own window integration, surfaces, or compositor policy.
//!
//! # Feature flags
//!
//! - `default`: Enables the `vello` module.
//! - `imaging_vello`: Enables the `vello` module and texture rendering support.
//! - `imaging_vello_hybrid`: Enables the `vello_hybrid` module and texture rendering support.
//! - `imaging_vello_cpu`: Enables the `vello_cpu` module for headless image rendering only.
//! - `imaging_skia`: Enables the `skia` module and texture rendering support on non-wasm targets.

// LINEBENDER LINT SET - lib.rs - v3
// See https://linebender.org/wiki/canonical-lints/
#![cfg_attr(not(test), warn(unused_crate_dependencies))]
#![warn(clippy::print_stdout, clippy::print_stderr)]
#![cfg_attr(target_pointer_width = "64", warn(clippy::trivially_copy_pass_by_ref))]
// END LINEBENDER LINT SET
#![cfg_attr(docsrs, feature(doc_cfg))]

use imaging::record::{Scene, ValidateError, replay_transformed};
use imaging::render::RenderSource;
use imaging::{PaintSink, Painter};
use kurbo::{Affine, Rect};
use peniko::Color;

#[cfg(all(
    not(target_arch = "wasm32"),
    any(feature = "imaging_vello", feature = "imaging_vello_hybrid")
))]
mod headless_wgpu;

/// Masonry helpers for rendering retained scenes with `imaging_skia`.
#[cfg(all(feature = "imaging_skia", not(target_arch = "wasm32")))]
pub mod skia;
/// Host-neutral texture rendering helpers for texture-capable backends.
#[cfg(not(target_arch = "wasm32"))]
pub mod texture_render;
/// Masonry helpers for rendering retained scenes with `imaging_vello`.
#[cfg(all(not(target_arch = "wasm32"), feature = "imaging_vello"))]
pub mod vello;
/// Masonry helpers for rendering retained scenes with `imaging_vello_cpu`.
#[cfg(feature = "imaging_vello_cpu")]
pub mod vello_cpu;
/// Masonry helpers for rendering retained scenes with `imaging_vello_hybrid`.
#[cfg(all(not(target_arch = "wasm32"), feature = "imaging_vello_hybrid"))]
pub mod vello_hybrid;

pub use imaging::render::ImageRenderer;
#[cfg(not(target_arch = "wasm32"))]
pub use imaging_wgpu::TextureRenderer;

/// Backend-selected helpers for headless image rendering.
pub mod image_render {
    #[cfg(all(
        not(target_arch = "wasm32"),
        not(feature = "imaging_vello"),
        feature = "imaging_skia"
    ))]
    pub use crate::skia::{BACKEND_NAME, Renderer, new_headless_renderer};
    #[cfg(all(not(target_arch = "wasm32"), feature = "imaging_vello"))]
    pub use crate::vello::{BACKEND_NAME, Renderer, new_headless_renderer};
    #[cfg(all(
        not(feature = "imaging_vello"),
        not(feature = "imaging_skia"),
        not(feature = "imaging_vello_hybrid"),
        feature = "imaging_vello_cpu"
    ))]
    pub use crate::vello_cpu::{BACKEND_NAME, Renderer, new_headless_renderer};
    #[cfg(all(
        not(target_arch = "wasm32"),
        not(feature = "imaging_vello"),
        not(feature = "imaging_skia"),
        feature = "imaging_vello_hybrid"
    ))]
    pub use crate::vello_hybrid::{BACKEND_NAME, Renderer, new_headless_renderer};

    #[cfg(not(any(
        feature = "imaging_vello",
        feature = "imaging_vello_hybrid",
        feature = "imaging_vello_cpu",
        all(feature = "imaging_skia", not(target_arch = "wasm32"))
    )))]
    pub use self::no_backend::{BACKEND_NAME, Error, Renderer, new_headless_renderer};

    #[cfg(not(any(
        feature = "imaging_vello",
        feature = "imaging_vello_hybrid",
        feature = "imaging_vello_cpu",
        all(feature = "imaging_skia", not(target_arch = "wasm32"))
    )))]
    mod no_backend {
        use imaging::render::{
            ImageBufferFormat, ImageBufferTarget, ImageRendererError, RenderSource,
        };

        /// Error returned when no image-render backend feature is enabled.
        #[derive(Debug)]
        pub struct Error;

        impl core::fmt::Display for Error {
            fn fmt(&self, f: &mut core::fmt::Formatter<'_>) -> core::fmt::Result {
                f.write_str("no imaging backend feature selected")
            }
        }

        impl std::error::Error for Error {}

        /// Placeholder renderer used when no image-render backend feature is enabled.
        #[derive(Debug)]
        pub struct Renderer;

        /// Stable diagnostics name for the backend-less stub renderer.
        pub const BACKEND_NAME: &str = "no_backend";

        /// Create the backend-less stub renderer.
        pub fn new_headless_renderer() -> Result<Renderer, Error> {
            Err(Error)
        }

        impl imaging::render::ImageRenderer for Renderer {
            fn supported_image_formats(&self) -> Vec<ImageBufferFormat> {
                Vec::new()
            }

            fn render_source_into(
                &mut self,
                _: &mut dyn RenderSource,
                _: ImageBufferTarget<'_>,
            ) -> Result<(), ImageRendererError> {
                Err(ImageRendererError::backend(Error))
            }
        }
    }
}

/// A Masonry overlay layer ready to be composited into window space.
#[derive(Clone, Copy)]
pub struct Layer<'a> {
    /// The retained scene for this layer in layer-local coordinates.
    pub scene: &'a Scene,
    /// Transform from layer-local coordinates into window coordinates.
    pub transform: Affine,
}

impl core::fmt::Debug for Layer<'_> {
    fn fmt(&self, f: &mut core::fmt::Formatter<'_>) -> core::fmt::Result {
        f.debug_struct("Layer")
            .field("scene", &"(Scene)")
            .field("transform", &self.transform)
            .finish()
    }
}

/// A flattened Masonry frame ready to be adapted to a concrete render target.
///
/// This is intentionally a single-target convenience type for Masonry's current rendering paths.
/// Future compositor-oriented work is expected to preserve more layer structure above this level.
#[derive(Clone, Copy, Debug)]
pub struct PreparedFrame<'a> {
    /// Frame width in physical pixels.
    pub width: u32,
    /// Frame height in physical pixels.
    pub height: u32,
    /// Window scale factor.
    pub scale_factor: f64,
    /// Background color to paint before replaying scene content.
    pub background_color: Color,
    /// Base retained scene in root coordinates.
    pub base: &'a Scene,
    /// Overlay layers in painter order.
    pub overlays: &'a [Layer<'a>],
}

impl<'a> PreparedFrame<'a> {
    /// Create a flattened Masonry frame from base content plus overlays.
    pub fn new(
        width: u32,
        height: u32,
        scale_factor: f64,
        background_color: Color,
        base: &'a Scene,
        overlays: &'a [Layer<'a>],
    ) -> Self {
        Self {
            width,
            height,
            scale_factor,
            background_color,
            base,
            overlays,
        }
    }
}

impl RenderSource for PreparedFrame<'_> {
    fn validate(&self) -> Result<(), ValidateError> {
        validate_layers(self.base, self.overlays)
    }

    fn paint_into(&mut self, sink: &mut dyn PaintSink) {
        {
            let mut painter = Painter::new(sink);
            painter.fill_rect(
                Rect::new(0.0, 0.0, f64::from(self.width), f64::from(self.height)),
                self.background_color,
            );
        }

        let scale = Affine::scale(self.scale_factor);
        replay_transformed(self.base, sink, scale);
        for layer in self.overlays {
            replay_transformed(layer.scene, sink, scale * layer.transform);
        }
    }
}

fn validate_layers(base: &Scene, overlays: &[Layer<'_>]) -> Result<(), ValidateError> {
    base.validate()?;
    for layer in overlays {
        layer.scene.validate()?;
    }
    Ok(())
}
