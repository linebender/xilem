// Copyright 2025 the Xilem Authors
// SPDX-License-Identifier: Apache-2.0

use crate::kurbo::{Point, Rect};

#[derive(Clone, Copy, Debug, PartialEq)]
/// A point with coordinates in the range [0.0, 1.0].
///
/// This is useful for specifying points in a normalized space, such as a gradient.
pub struct UnitPoint {
    u: f64,
    v: f64,
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

    /// Creates a new `UnitPoint`.
    ///
    /// The `u` and `v` coordinates describe the point, with (0.0, 0.0) being
    /// the top-left, and (1.0, 1.0) being the bottom-right.
    pub const fn new(u: f64, v: f64) -> Self {
        Self { u, v }
    }

    /// Given a rectangle, resolves the point within the rectangle.
    ///
    /// For aligning oversized items you should give this a reversed rect.
    /// That is, if you're resolving the origin of a 15x15 child inside a 10x10 container,
    /// the rect should be `(0, 0, -5, -5)`. Then `TOP_LEFT` will resolve to `(0, 0)`
    /// while `BOTTOM_RIGHT` will resolve to `(-5, -5)`.
    pub const fn resolve(self, rect: Rect) -> Point {
        Point::new(
            rect.x0 + self.u * (rect.x1 - rect.x0),
            rect.y0 + self.v * (rect.y1 - rect.y0),
        )
    }
}
