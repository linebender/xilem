use glazier::kurbo::{self, Rect, Shape};
use piet_scene::{
    Affine, Brush, Cap, ExtendMode, Fill, GradientStops, Join, LinearGradient, PathElement, Point,
    SceneBuilder, Stroke,
};

#[derive(Debug, Clone, Copy)]
pub struct UnitPoint {
    u: f64,
    v: f64,
}

pub fn stroke(builder: &mut SceneBuilder, path: &impl Shape, brush: &Brush, stroke_width: f64) {
    let style = Stroke {
        width: stroke_width as f32,
        join: Join::Round,
        miter_limit: 1.0,
        start_cap: Cap::Round,
        end_cap: Cap::Round,
        dash_pattern: [],
        dash_offset: 0.0,
        scale: false,
    };
    // TODO: figure out how to avoid allocation
    // (Just removing the collect should work in theory, but running into a clone bound)
    let elements = path
        .path_elements(1e-3)
        .map(PathElement::from_kurbo)
        .collect::<Vec<_>>();
    builder.stroke(&style, Affine::IDENTITY, brush, None, &elements)
}

// Note: copied from piet
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
    builder: &mut SceneBuilder,
    path: &impl Shape,
    stops: GradientStops,
    start: UnitPoint,
    end: UnitPoint,
) {
    let rect = path.bounding_box();
    let lin_grad = LinearGradient {
        start: Point::from_kurbo(start.resolve(rect)),
        end: Point::from_kurbo(end.resolve(rect)),
        stops,
        extend: ExtendMode::Pad,
    };
    let elements = path
        .path_elements(1e-3)
        .map(PathElement::from_kurbo)
        .collect::<Vec<_>>();
    builder.fill(
        Fill::NonZero,
        Affine::IDENTITY,
        &Brush::LinearGradient(lin_grad),
        None,
        elements,
    );
}
