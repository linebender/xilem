// Copyright 2020 the Xilem Authors and the Druid Authors
// SPDX-License-Identifier: Apache-2.0

//! Miscellaneous utility functions.

use std::any::Any;
use std::hash::Hash;

use vello::Scene;
use vello::kurbo::Join;
use vello::kurbo::{
    Affine, Rect, Shape, Stroke, {self},
};
use vello::peniko::{BrushRef, Color, ColorStopsSource, Fill, Gradient};

/// Panic in debug and `tracing::error` in release mode.
///
/// This macro is in some way a combination of `panic` and `debug_assert`,
/// but it will log the provided message instead of ignoring it in release builds.
///
/// It's useful when a backtrace would aid debugging but a crash can be avoided in release.
#[macro_export]
macro_rules! debug_panic {
    ($msg:expr$(,)?) => {
        if cfg!(debug_assertions) {
            panic!($msg);
        } else {
            tracing::error!($msg);
        }
    };
    ($fmt:expr, $($arg:tt)+) => {
        if cfg!(debug_assertions) {
            panic!($fmt, $($arg)*);
        } else {
            tracing::error!($fmt, $($arg)*);
        }
    };
}

// ---

pub(crate) type AnyMap = anymap3::Map<dyn Any + Send + Sync>;

// ---

/// An enum for specifying whether an event was handled.
#[derive(Clone, Copy, Debug, Eq, Hash, PartialEq)]
pub enum Handled {
    /// An event was already handled, and shouldn't be propagated to other event handlers.
    Yes,
    /// An event has not yet been handled.
    No,
}

impl Handled {
    /// Has the event been handled yet?
    pub fn is_handled(self) -> bool {
        self == Self::Yes
    }
}

impl From<bool> for Handled {
    /// Returns `Handled::Yes` if `handled` is true, and `Handled::No` otherwise.
    fn from(handled: bool) -> Self {
        if handled { Self::Yes } else { Self::No }
    }
}

// --- MARK: PAINT HELPERS

#[derive(Debug, Clone, Copy)]
/// A point with coordinates in the range [0.0, 1.0].
///
/// This is useful for specifying points in a normalized space, such as a gradient.
pub struct UnitPoint {
    u: f64,
    v: f64,
}

#[expect(
    single_use_lifetimes,
    reason = "Anonymous lifetimes in `impl Trait` are unstable, see https://github.com/rust-lang/rust/issues/129255"
)]
/// Helper function for [`Scene::stroke`].
pub fn stroke<'b>(
    scene: &mut Scene,
    path: &impl Shape,
    brush: impl Into<BrushRef<'b>>,
    stroke_width: f64,
) {
    // Using Join::Miter avoids rounding corners when a widget has a wide border.
    let style = Stroke {
        width: stroke_width,
        join: Join::Miter,
        ..Default::default()
    };
    scene.stroke(&style, Affine::IDENTITY, brush, None, path);
}

impl UnitPoint {
    /// `(0.0, 0.0)`
    pub const TOP_LEFT: Self = Self::new(0.0, 0.0);
    /// `(0.5, 0.0)`
    pub const TOP: Self = Self::new(0.5, 0.0);
    /// `(1.0, 0.0)`
    pub const TOP_RIGHT: Self = Self::new(1.0, 0.0);
    /// `(0.0, 0.5)`
    pub const LEFT: Self = Self::new(0.0, 0.5);
    /// `(0.5, 0.5)`
    pub const CENTER: Self = Self::new(0.5, 0.5);
    /// `(1.0, 0.5)`
    pub const RIGHT: Self = Self::new(1.0, 0.5);
    /// `(0.0, 1.0)`
    pub const BOTTOM_LEFT: Self = Self::new(0.0, 1.0);
    /// `(0.5, 1.0)`
    pub const BOTTOM: Self = Self::new(0.5, 1.0);
    /// `(1.0, 1.0)`
    pub const BOTTOM_RIGHT: Self = Self::new(1.0, 1.0);

    /// Create a new `UnitPoint`.
    ///
    /// The `u` and `v` coordinates describe the point, with (0.0, 0.0) being
    /// the top-left, and (1.0, 1.0) being the bottom-right.
    pub const fn new(u: f64, v: f64) -> Self {
        Self { u, v }
    }

    /// Given a rectangle, resolve the point within the rectangle.
    pub fn resolve(self, rect: Rect) -> kurbo::Point {
        kurbo::Point::new(
            rect.x0 + self.u * (rect.x1 - rect.x0),
            rect.y0 + self.v * (rect.y1 - rect.y0),
        )
    }
}

#[expect(
    single_use_lifetimes,
    reason = "Anonymous lifetimes in `impl Trait` are unstable, see https://github.com/rust-lang/rust/issues/129255"
)]
/// Helper function for [`Scene::fill`].
pub fn fill<'b>(scene: &mut Scene, path: &impl Shape, brush: impl Into<BrushRef<'b>>) {
    scene.fill(Fill::NonZero, Affine::IDENTITY, brush, None, path);
}

/// Helper function for [`Scene::fill`] with a linear gradient as the brush.
pub fn fill_lin_gradient(
    scene: &mut Scene,
    path: &impl Shape,
    stops: impl ColorStopsSource,
    start: UnitPoint,
    end: UnitPoint,
) {
    let rect = path.bounding_box();
    let brush = Gradient::new_linear(start.resolve(rect), end.resolve(rect)).with_stops(stops);
    scene.fill(Fill::NonZero, Affine::IDENTITY, &brush, None, path);
}

/// Helper function for [`Scene::fill`] with a uniform color as the brush.
pub fn fill_color(scene: &mut Scene, path: &impl Shape, color: Color) {
    scene.fill(Fill::NonZero, Affine::IDENTITY, color, None, path);
}

// ---

static DEBUG_COLOR: &[Color] = &[
    Color::from_rgb8(230, 25, 75),
    Color::from_rgb8(60, 180, 75),
    Color::from_rgb8(255, 225, 25),
    Color::from_rgb8(0, 130, 200),
    Color::from_rgb8(245, 130, 48),
    Color::from_rgb8(70, 240, 240),
    Color::from_rgb8(240, 50, 230),
    Color::from_rgb8(250, 190, 190),
    Color::from_rgb8(0, 128, 128),
    Color::from_rgb8(230, 190, 255),
    Color::from_rgb8(170, 110, 40),
    Color::from_rgb8(255, 250, 200),
    Color::from_rgb8(128, 0, 0),
    Color::from_rgb8(170, 255, 195),
    Color::from_rgb8(0, 0, 128),
    Color::from_rgb8(128, 128, 128),
    Color::from_rgb8(255, 255, 255),
    Color::from_rgb8(0, 0, 0),
];

/// A color used for debug painting.
///
/// The same color is always returned given the same id, usually the id of a widget.
/// When painting a widget, [`PaintCtx::debug_color`][crate::core::PaintCtx::debug_color] is typically used instead.
pub fn get_debug_color(id: u64) -> Color {
    let color_num = id as usize % DEBUG_COLOR.len();
    DEBUG_COLOR[color_num]
}

// ---

// FIXME - We're essentially completely disabling screenshots, period.
// Hopefully we'll be able to re-enable them soon.
// See https://github.com/linebender/xilem/issues/851

#[doc(hidden)]
#[macro_export]
macro_rules! include_screenshot {
    ($path:literal $(, $caption:literal)? $(,)?) => {
        // On docsrs we just remove the screenshot links for now.
        " "
    };
}

// TODO - Re-enable this once we find a way to load screenshots that doesn't go against our
// storage quotas.
#[cfg(FALSE)]
#[cfg(docsrs)]
#[doc(hidden)]
#[macro_export]
macro_rules! include_screenshot {
    ($path:literal $(, $caption:literal)? $(,)?) => {
        concat!(
            "![", $($caption,)? "]",
            "(", "https://media.githubusercontent.com/media/linebender/xilem/",
            "masonry-v", env!("CARGO_PKG_VERSION"), "/masonry/screenshots/", $path,
            ")",
        )
    };
}

#[cfg(FALSE)]
#[cfg(not(docsrs))]
#[doc(hidden)]
#[macro_export]
/// Macro used to create markdown img tag, with a different URL when uploading to docs.rs.
macro_rules! include_screenshot {
    ($path:literal $(, $caption:literal)? $(,)?) => {
        // This space at the start avoids triggering https://rust-lang.github.io/rust-clippy/master/index.html#suspicious_doc_comments
        // when using this macro in a `doc` attribute
        concat!(
            " ![", $($caption,)? "]",
            "(", env!("CARGO_MANIFEST_DIR"), "/screenshots/", $path, ")",
        )
    };
}
