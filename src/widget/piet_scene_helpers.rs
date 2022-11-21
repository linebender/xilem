
use glazier::kurbo::Shape;
use piet_scene::{SceneBuilder, Stroke, Join, Cap, Brush, Affine, PathElement};

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
    let elements =  path.path_elements(1e-3).map(PathElement::from_kurbo).collect::<Vec<_>>();
    builder.stroke(&style, Affine::IDENTITY, brush, None, &elements)
}
