// Copyright 2020 the Xilem Authors and the Druid Authors
// SPDX-License-Identifier: Apache-2.0

//! Miscellaneous utility functions.

use vello::Scene;
use vello::kurbo::{Affine, Join, Shape, Stroke};
use vello::peniko::{BrushRef, Color, Fill};

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

use crate::core::{Widget, WidgetId};
pub use crate::debug_panic;

/// Provides sanitization of values.
///
/// This is a generic trait that doesn't specify what sanitization exactly means,
/// as that will be implementations specific per type.
///
/// Right now it is also implemented for `f64` and `Option<f64>` in a way
/// where it forbids non-finite and negative values. This is immediately useful for
/// Masonry itself but is likely to go away as we migrate our float usage to newtypes.
pub trait Sanitize {
    /// Returns the sanitized value.
    ///
    /// Generally should remove all invariants.
    /// Depending on the implementation, may also panic or log.
    ///
    /// See the specific implementation docs for more details.
    #[track_caller]
    fn sanitize(self, name: &str) -> Self;
}

impl Sanitize for f64 {
    /// Ensures the value is finite and non-negative.
    ///
    /// Non-finite or negative value falls back to zero.
    ///
    /// `name` is how the value will be named in the log message.
    ///
    /// # Panics
    ///
    /// Panics if the value is non-finite or negative and debug assertions are enabled.
    #[track_caller]
    fn sanitize(self, name: &str) -> Self {
        if !self.is_finite() {
            debug_panic!("{name} must be finite. Received: {self}");
            0.
        } else if self < 0. {
            debug_panic!("{name} must be non-negative. Received: {self}");
            0.
        } else {
            self
        }
    }
}

impl Sanitize for Option<f64> {
    /// Ensures the value is finite and non-negative.
    ///
    /// Non-finite or negative value falls back to `None`.
    ///
    /// `name` is how the value will be named in the log message.
    ///
    /// # Panics
    ///
    /// Panics if the value is non-finite or negative and debug assertions are enabled.
    #[track_caller]
    fn sanitize(self, name: &str) -> Self {
        match self {
            Some(val) => {
                if !val.is_finite() {
                    debug_panic!("{name} must be finite. Received: {val}");
                    None
                } else if val < 0. {
                    debug_panic!("{name} must be non-negative. Received: {val}");
                    None
                } else {
                    Some(val)
                }
            }
            None => None,
        }
    }
}

// ---

pub(crate) type AnyMap = anymap3::Map<dyn anymap3::CloneAny + Send + Sync>;
pub(crate) type TypeSet = std::collections::HashSet<
    std::any::TypeId,
    std::hash::BuildHasherDefault<anymap3::TypeIdHasher>,
>;

// --- MARK: PAINT HELPERS

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

#[expect(
    single_use_lifetimes,
    reason = "Anonymous lifetimes in `impl Trait` are unstable, see https://github.com/rust-lang/rust/issues/129255"
)]
/// Helper function for [`Scene::fill`].
pub fn fill<'b>(scene: &mut Scene, path: &impl Shape, brush: impl Into<BrushRef<'b>>) {
    scene.fill(Fill::NonZero, Affine::IDENTITY, brush, None, path);
}

/// Helper function for [`Scene::fill`] with a uniform color as the brush.
pub fn fill_color(scene: &mut Scene, path: &impl Shape, color: Color) {
    scene.fill(Fill::NonZero, Affine::IDENTITY, color, None, path);
}

// ---

/// Convert a 2d rectangle from Parley to one used for drawing in Vello and other maths.
pub fn bounding_box_to_rect(bb: parley::BoundingBox) -> vello::kurbo::Rect {
    vello::kurbo::Rect {
        x0: bb.x0,
        y0: bb.y0,
        x1: bb.x1,
        y1: bb.y1,
    }
}

// ---

#[cfg(not(target_arch = "wasm32"))]
pub use std::time::{Duration, Instant};

#[cfg(target_arch = "wasm32")]
pub use web_time::{Duration, Instant};

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

pub(crate) struct ParentLinkedList<'a> {
    pub(crate) widget: &'a dyn Widget,
    pub(crate) id: WidgetId,
    pub(crate) parent: Option<&'a Self>,
}
