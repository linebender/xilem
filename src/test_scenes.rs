use super::text::*;
use parley::FontContext;
use piet_scene::*;

pub fn render(fcx: &mut FontContext, scene: &mut Scene, which: usize, arg: u64) {
    match which {
        _ => basic_scene(fcx, scene, arg),
    }
}

fn basic_scene(fcx: &mut FontContext, scene: &mut Scene, arg: u64) {
    let transform = Affine::translate(400.0, 400.0) * Affine::rotate((arg as f64 * 0.01) as f32);
    let mut builder = SceneBuilder::for_scene(scene);
    let stops = &[
        GradientStop {
            offset: 0.0,
            color: Color::rgb8(128, 0, 0),
        },
        GradientStop {
            offset: 0.5,
            color: Color::rgb8(0, 128, 0),
        },
        GradientStop {
            offset: 1.0,
            color: Color::rgb8(0, 0, 128),
        },
    ][..];
    let gradient = Brush::LinearGradient(LinearGradient {
        start: Point::new(0.0, 0.0),
        end: Point::new(0.0, 400.0),
        extend: ExtendMode::Pad,
        stops: stops.iter().copied().collect(),
    });
    builder.fill(
        Fill::NonZero,
        transform,
        &gradient,
        None,
        Rect {
            min: Point::new(0.0, 0.0),
            max: Point::new(600.0, 400.0),
        }
        .elements(),
    );
    let scale = (arg as f64 * 0.01).sin() * 0.5 + 1.5;
    let mut lcx = parley::LayoutContext::new();
    let mut layout_builder =
        lcx.ranged_builder(fcx, "Hello piet-gpu! ഹലോ ਸਤ ਸ੍ਰੀ ਅਕਾਲ مرحبا!", scale as f32);
    layout_builder.push_default(&parley::style::StyleProperty::FontSize(34.0));
    layout_builder.push(
        &parley::style::StyleProperty::Brush(ParleyBrush(Brush::Solid(Color::rgb8(255, 255, 0)))),
        6..10,
    );
    layout_builder.push(&parley::style::StyleProperty::FontSize(48.0), 6..10);
    layout_builder.push_default(&parley::style::StyleProperty::Brush(ParleyBrush(
        Brush::Solid(Color::rgb8(255, 255, 255)),
    )));
    let mut layout = layout_builder.build();
    layout.break_all_lines(None, parley::layout::Alignment::Start);
    render_text(&mut builder, Affine::translate(100.0, 400.0), &layout);
    builder.finish();
}
