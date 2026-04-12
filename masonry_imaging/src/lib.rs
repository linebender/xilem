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
//! - preparing ordered Masonry render layers from base content plus overlays
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
use masonry_core::app::{ExternalLayerKind, VisualLayerBoundary, VisualLayerKind, VisualLayerPlan};
use peniko::Color;

#[cfg(any(feature = "imaging_vello", feature = "imaging_vello_hybrid"))]
mod headless_wgpu;

/// Masonry helpers for rendering retained scenes with `imaging_skia`.
#[cfg(all(feature = "imaging_skia", not(target_arch = "wasm32")))]
pub mod skia;
/// Host-neutral texture rendering helpers for texture-capable backends.
pub mod texture_render;
/// Masonry helpers for rendering retained scenes with `imaging_vello`.
#[cfg(feature = "imaging_vello")]
pub mod vello;
/// Masonry helpers for rendering retained scenes with `imaging_vello_cpu`.
#[cfg(feature = "imaging_vello_cpu")]
pub mod vello_cpu;
/// Masonry helpers for rendering retained scenes with `imaging_vello_hybrid`.
#[cfg(feature = "imaging_vello_hybrid")]
pub mod vello_hybrid;

pub use imaging::render::{ImageRenderer, TextureRenderer};

/// Backend-selected helpers for headless image rendering.
pub mod image_render {
    #[cfg(all(
        not(feature = "imaging_vello"),
        feature = "imaging_skia",
        not(target_arch = "wasm32")
    ))]
    pub use crate::skia::{BACKEND_NAME, Renderer, new_headless_renderer};
    #[cfg(feature = "imaging_vello")]
    pub use crate::vello::{BACKEND_NAME, Renderer, new_headless_renderer};
    #[cfg(all(
        not(feature = "imaging_vello"),
        not(feature = "imaging_skia"),
        not(feature = "imaging_vello_hybrid"),
        feature = "imaging_vello_cpu"
    ))]
    pub use crate::vello_cpu::{BACKEND_NAME, Renderer, new_headless_renderer};
    #[cfg(all(
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
        use imaging::{RgbaImage, render::RenderSource};

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
            type Error = Error;

            fn render_source_into<S: RenderSource + ?Sized>(
                &mut self,
                _: &mut S,
                _: u32,
                _: u32,
                _: &mut RgbaImage,
            ) -> Result<(), Self::Error> {
                Err(Error)
            }
        }
    }
}

/// Opaque reference to a host-owned external layer.
///
/// This is a placeholder for content such as a 3D viewport or platform-native compositor layer.
/// Current render-source adapters do not realize external layers; compositor-aware hosts are
/// expected to handle them directly from a higher-level render plan.
#[derive(Clone, Copy, Debug, PartialEq, Eq, Hash)]
pub struct ExternalLayerRef {
    /// Stable layer identifier chosen by the host/widget integration.
    pub id: u64,
    /// Logical kind of external layer requested by Masonry.
    pub kind: ExternalLayerKind,
}

/// The content realization for a prepared layer.
#[derive(Clone, Copy, Debug)]
pub enum LayerKind<'a> {
    /// Masonry-painted retained scene content.
    Scene(&'a Scene),
    /// Host-owned external/native content.
    External(ExternalLayerRef),
}

/// A Masonry render layer ready to be composited into window space.
#[derive(Clone, Copy)]
pub struct PreparedLayer<'a> {
    /// The content of this layer.
    pub kind: LayerKind<'a>,
    /// Where this layer boundary originated in the widget model.
    pub boundary: VisualLayerBoundary,
    /// Axis-aligned bounds of this layer's content in layer-local coordinates.
    pub bounds: Rect,
    /// Optional clip to apply in layer-local coordinates when realizing the layer.
    pub clip: Option<Rect>,
    /// Transform from layer-local coordinates into window coordinates.
    pub transform: Affine,
}

impl core::fmt::Debug for PreparedLayer<'_> {
    fn fmt(&self, f: &mut core::fmt::Formatter<'_>) -> core::fmt::Result {
        f.debug_struct("PreparedLayer")
            .field("kind", &self.kind)
            .field("boundary", &self.boundary)
            .field("bounds", &self.bounds)
            .field("clip", &self.clip)
            .field("transform", &self.transform)
            .finish()
    }
}

impl<'a> PreparedLayer<'a> {
    /// Create a Masonry-painted scene layer.
    pub fn scene(
        scene: &'a Scene,
        boundary: VisualLayerBoundary,
        bounds: Rect,
        clip: Option<Rect>,
        transform: Affine,
    ) -> Self {
        Self {
            kind: LayerKind::Scene(scene),
            boundary,
            bounds,
            clip,
            transform,
        }
    }

    /// Create a host-owned external layer placeholder.
    pub fn external(
        external: ExternalLayerRef,
        boundary: VisualLayerBoundary,
        bounds: Rect,
        clip: Option<Rect>,
        transform: Affine,
    ) -> Self {
        Self {
            kind: LayerKind::External(external),
            boundary,
            bounds,
            clip,
            transform,
        }
    }
}

/// Compatibility alias for the old layer name.
pub type Layer<'a> = PreparedLayer<'a>;

#[derive(Clone, Copy)]
enum LayerSource<'a> {
    Prepared(&'a [PreparedLayer<'a>]),
    Visual(&'a VisualLayerPlan),
}

impl core::fmt::Debug for LayerSource<'_> {
    fn fmt(&self, f: &mut core::fmt::Formatter<'_>) -> core::fmt::Result {
        match self {
            Self::Prepared(layers) => f.debug_tuple("Prepared").field(&layers.len()).finish(),
            Self::Visual(layers) => f.debug_tuple("Visual").field(&layers.layers.len()).finish(),
        }
    }
}

impl LayerSource<'_> {
    fn validate(self) -> Result<(), ValidateError> {
        match self {
            Self::Prepared(layers) => validate_prepared_layers(layers),
            Self::Visual(layers) => validate_visual_layers(layers),
        }
    }

    fn replay_into(self, sink: &mut dyn PaintSink, transform: Affine) {
        match self {
            Self::Prepared(layers) => {
                for layer in layers {
                    if let LayerKind::Scene(scene) = layer.kind {
                        replay_transformed(scene, sink, transform * layer.transform);
                    }
                }
            }
            Self::Visual(layers) => {
                for layer in &layers.layers {
                    if let VisualLayerKind::Scene(scene) = &layer.kind {
                        replay_transformed(scene, sink, transform * layer.transform);
                    }
                }
            }
        }
    }
}

/// Masonry render source for a window-sized frame.
#[derive(Clone, Copy, Debug)]
pub struct WindowSource<'a> {
    width: u32,
    height: u32,
    scale_factor: f64,
    background_color: Color,
    layers: LayerSource<'a>,
}

impl<'a> WindowSource<'a> {
    /// Create a render source directly from Masonry-imaging prepared layers.
    pub fn from_prepared_layers(
        width: u32,
        height: u32,
        scale_factor: f64,
        background_color: Color,
        layers: &'a [PreparedLayer<'a>],
    ) -> Self {
        Self {
            width,
            height,
            scale_factor,
            background_color,
            layers: LayerSource::Prepared(layers),
        }
    }

    /// Create a render source directly from Masonry visual layers.
    pub fn from_visual_layers(
        width: u32,
        height: u32,
        scale_factor: f64,
        background_color: Color,
        layers: &'a VisualLayerPlan,
    ) -> Self {
        Self {
            width,
            height,
            scale_factor,
            background_color,
            layers: LayerSource::Visual(layers),
        }
    }
}

impl RenderSource for WindowSource<'_> {
    fn validate(&self) -> Result<(), ValidateError> {
        self.layers.validate()
    }

    fn paint_into(&mut self, sink: &mut dyn PaintSink) {
        {
            let mut painter = Painter::new(sink);
            painter.fill_rect(
                Rect::new(0.0, 0.0, f64::from(self.width), f64::from(self.height)),
                self.background_color,
            );
        }

        self.layers
            .replay_into(sink, Affine::scale(self.scale_factor));
    }
}

/// Masonry render source for screenshot-style output with optional root padding.
#[derive(Clone, Copy, Debug)]
pub struct SnapshotSource<'a> {
    background_color: Color,
    layers: LayerSource<'a>,
    width: u32,
    height: u32,
    root_padding: u32,
}

impl<'a> SnapshotSource<'a> {
    /// Create a screenshot render source directly from Masonry-imaging prepared layers.
    pub fn from_prepared_layers(
        width: u32,
        height: u32,
        scale_factor: f64,
        background_color: Color,
        layers: &'a [PreparedLayer<'a>],
        root_padding: u32,
    ) -> Self {
        Self::from_parts(
            width,
            height,
            scale_factor,
            background_color,
            LayerSource::Prepared(layers),
            root_padding,
        )
    }

    fn from_parts(
        width: u32,
        height: u32,
        _scale_factor: f64,
        background_color: Color,
        layers: LayerSource<'a>,
        root_padding: u32,
    ) -> Self {
        let (width, height) = padded_dimensions(width, height, root_padding);
        Self {
            background_color,
            layers,
            width,
            height,
            root_padding,
        }
    }

    /// Create a screenshot render source directly from Masonry visual layers.
    pub fn from_visual_layers(
        width: u32,
        height: u32,
        scale_factor: f64,
        background_color: Color,
        layers: &'a VisualLayerPlan,
        root_padding: u32,
    ) -> Self {
        Self::from_parts(
            width,
            height,
            scale_factor,
            background_color,
            LayerSource::Visual(layers),
            root_padding,
        )
    }

    /// Output width in pixels.
    pub fn width(&self) -> u32 {
        self.width
    }

    /// Output height in pixels.
    pub fn height(&self) -> u32 {
        self.height
    }
}

impl RenderSource for SnapshotSource<'_> {
    fn validate(&self) -> Result<(), ValidateError> {
        self.layers.validate()
    }

    fn paint_into(&mut self, sink: &mut dyn PaintSink) {
        {
            let mut painter = Painter::new(sink);
            painter.fill_rect(
                Rect::new(0.0, 0.0, f64::from(self.width), f64::from(self.height)),
                self.background_color,
            );

            if self.root_padding != 0 {
                // 25% opacity of 50% grey provides a border of where the actual widget content is.
                // Alternatively, maybe we should use a stronger color here?
                let padding_color = Color::from_rgba8(127, 127, 127, 64);
                // We draw the border first, so that any content is above the background color.
                for [x0, y0, x1, y1] in padding_rects(self.width, self.height, self.root_padding) {
                    painter.fill_rect(
                        Rect::new(x0 as f64, y0 as f64, x1 as f64, y1 as f64),
                        padding_color,
                    );
                }
            }
        }

        let padding_transform =
            Affine::translate((f64::from(self.root_padding), f64::from(self.root_padding)));
        self.layers.replay_into(sink, padding_transform);
    }
}

/// Compute the output dimensions for a padded render target.
fn padded_dimensions(content_width: u32, content_height: u32, root_padding: u32) -> (u32, u32) {
    // Avoid having a zero-sized image.
    let width = content_width.max(1) + root_padding * 2;
    let height = content_height.max(1) + root_padding * 2;
    (width, height)
}

fn padding_rects(width: u32, height: u32, padding: u32) -> [[u32; 4]; 4] {
    [
        [0, 0, padding, height],                              // Left edge
        [width - padding, 0, width, height],                  // Right edge
        [padding, 0, width - padding, padding],               // Top edge
        [padding, height - padding, width - padding, height], // Bottom edge
    ]
}

fn validate_prepared_layers(layers: &[PreparedLayer<'_>]) -> Result<(), ValidateError> {
    for layer in layers {
        if let LayerKind::Scene(scene) = layer.kind {
            scene.validate()?;
        }
    }
    Ok(())
}

fn validate_visual_layers(layers: &VisualLayerPlan) -> Result<(), ValidateError> {
    for layer in &layers.layers {
        if let VisualLayerKind::Scene(scene) = &layer.kind {
            scene.validate()?;
        }
    }
    Ok(())
}

#[cfg(test)]
mod tests {
    use masonry_core::app::VisualLayerPlan;
    use peniko::Color;

    use super::{SnapshotSource, padded_dimensions};

    #[test]
    fn padded_dimensions_avoid_zero() {
        assert_eq!(padded_dimensions(0, 0, 0), (1, 1));
        assert_eq!(padded_dimensions(0, 2, 5), (11, 12));
    }

    #[test]
    fn snapshot_source_from_visual_layers_uses_padded_dimensions() {
        let layers = VisualLayerPlan { layers: Vec::new() };
        let source = SnapshotSource::from_visual_layers(0, 2, 1.0, Color::WHITE, &layers, 5);

        assert_eq!(source.width(), 11);
        assert_eq!(source.height(), 12);
        assert!(imaging::render::RenderSource::validate(&source).is_ok());
    }
}
