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

// ---

/// Trait extending Any, implemented for all types that implement Any.
///
/// This is a band-aid to substitute for a lack of dyn trait upcasting.
pub trait AsAny: Any {
    /// Return self.
    fn as_dyn_any(&self) -> &dyn Any;
    /// Return self.
    fn as_mut_dyn_any(&mut self) -> &mut dyn Any;
}

impl<T: Any> AsAny for T {
    fn as_dyn_any(&self) -> &dyn Any {
        self
    }

    fn as_mut_dyn_any(&mut self) -> &mut dyn Any {
        self
    }
}

// --- MARK: PAINT HELPERS ---

#[derive(Debug, Clone, Copy)]
#[allow(missing_docs)]
pub struct UnitPoint {
    u: f64,
    v: f64,
}

#[expect(
    single_use_lifetimes,
    reason = "Anonymous lifetimes in `impl Trait` are unstable, see https://github.com/rust-lang/rust/issues/129255"
)]
#[allow(missing_docs)]
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

#[allow(unused)]
#[allow(missing_docs)]
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

#[allow(missing_docs)]
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

#[allow(missing_docs)]
pub fn fill_color(scene: &mut Scene, path: &impl Shape, color: Color) {
    scene.fill(Fill::NonZero, Affine::IDENTITY, color, None, path);
}
