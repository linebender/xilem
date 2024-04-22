#![allow(missing_docs)]

use vello::{
    kurbo::{self, Affine, Rect, Shape, Stroke},
    peniko::{BrushRef, Color, ColorStopsSource, Fill, Gradient},
    Scene,
};

// TODO - Remove this file

#[derive(Debug, Clone, Copy)]
pub struct UnitPoint {
    u: f64,
    v: f64,
}

pub fn stroke<'b>(
    scene: &mut Scene,
    path: &impl Shape,
    brush: impl Into<BrushRef<'b>>,
    stroke_width: f64,
) {
    scene.stroke(
        &Stroke::new(stroke_width),
        Affine::IDENTITY,
        brush,
        None,
        path,
    );
}

#[allow(unused)]
impl UnitPoint {
    /// `(0.0, 0.0)`
    pub const TOP_LEFT: UnitPoint = UnitPoint::new(0.0, 0.0);
    /// `(0.5, 0.0)`
    pub const TOP: UnitPoint = UnitPoint::new(0.5, 0.0);
    /// `(1.0, 0.0)`
    pub const TOP_RIGHT: UnitPoint = UnitPoint::new(1.0, 0.0);
    /// `(0.0, 0.5)`
    pub const LEFT: UnitPoint = UnitPoint::new(0.0, 0.5);
    /// `(0.5, 0.5)`
    pub const CENTER: UnitPoint = UnitPoint::new(0.5, 0.5);
    /// `(1.0, 0.5)`
    pub const RIGHT: UnitPoint = UnitPoint::new(1.0, 0.5);
    /// `(0.0, 1.0)`
    pub const BOTTOM_LEFT: UnitPoint = UnitPoint::new(0.0, 1.0);
    /// `(0.5, 1.0)`
    pub const BOTTOM: UnitPoint = UnitPoint::new(0.5, 1.0);
    /// `(1.0, 1.0)`
    pub const BOTTOM_RIGHT: UnitPoint = UnitPoint::new(1.0, 1.0);

    /// Create a new UnitPoint.
    ///
    /// The `u` and `v` coordinates describe the point, with (0.0, 0.0) being
    /// the top-left, and (1.0, 1.0) being the bottom-right.
    pub const fn new(u: f64, v: f64) -> UnitPoint {
        UnitPoint { u, v }
    }

    /// Given a rectangle, resolve the point within the rectangle.
    pub fn resolve(self, rect: Rect) -> kurbo::Point {
        kurbo::Point::new(
            rect.x0 + self.u * (rect.x1 - rect.x0),
            rect.y0 + self.v * (rect.y1 - rect.y0),
        )
    }
}

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

pub fn fill_color(scene: &mut Scene, path: &impl Shape, color: Color) {
    scene.fill(Fill::NonZero, Affine::IDENTITY, color, None, path);
}
