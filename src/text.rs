use parley::Layout;
use vello::{
    kurbo::Affine,
    peniko::{Brush, Fill},
    SceneBuilder,
};

pub fn render_text(builder: &mut SceneBuilder, transform: Affine, layout: &Layout<Brush>) {
    for line in layout.lines() {
        for glyph_run in line.glyph_runs() {
            let mut x = glyph_run.offset();
            let y = glyph_run.baseline();
            let run = glyph_run.run();
            let font = run.font();
            let font_size = run.font_size();
            let font = vello::peniko::Font::new(font.data().0.clone(), font.index());
            let style = glyph_run.style();
            let coords = run
                .normalized_coords()
                .iter()
                .map(|coord| vello::skrifa::instance::NormalizedCoord::from_bits(*coord))
                .collect::<Vec<_>>();
            builder
                .draw_glyphs(&font)
                .brush(&style.brush)
                .transform(transform)
                .font_size(font_size)
                .normalized_coords(&coords)
                .draw(
                    Fill::NonZero,
                    glyph_run.glyphs().map(|glyph| {
                        let gx = x + glyph.x;
                        let gy = y - glyph.y;
                        x += glyph.advance;
                        vello::glyph::Glyph {
                            id: glyph.id as _,
                            x: gx,
                            y: gy,
                        }
                    }),
                );
        }
    }
}
