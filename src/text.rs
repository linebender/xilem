use parley::Layout;
use vello::kurbo::Affine;
use vello::{
    glyph::{
        pinot::{types::Tag, FontRef},
        GlyphContext,
    },
    peniko::{Brush, Color},
    *,
};

#[derive(Clone, PartialEq, Debug)]
pub struct ParleyBrush(pub Brush);

impl Default for ParleyBrush {
    fn default() -> ParleyBrush {
        ParleyBrush(Brush::Solid(Color::rgb8(0, 0, 0)))
    }
}

impl parley::style::Brush for ParleyBrush {}

pub fn render_text(builder: &mut SceneBuilder, transform: Affine, layout: &Layout<ParleyBrush>) {
    let mut gcx = GlyphContext::new();
    for line in layout.lines() {
        for glyph_run in line.glyph_runs() {
            let mut x = glyph_run.offset();
            let y = glyph_run.baseline();
            let run = glyph_run.run();
            let font = run.font().as_ref();
            let font_size = run.font_size();
            let font_ref = FontRef {
                data: font.data,
                offset: font.offset,
            };
            let style = glyph_run.style();
            let vars: [(Tag, f32); 0] = [];
            let mut gp = gcx.new_provider(&font_ref, None, font_size, false, vars);
            for glyph in glyph_run.glyphs() {
                if let Some(fragment) = gp.get(glyph.id, Some(&style.brush.0)) {
                    let gx = x + glyph.x;
                    let gy = y - glyph.y;
                    let xform = Affine::translate((gx as f64, gy as f64))
                        * Affine::scale_non_uniform(1.0, -1.0);
                    builder.append(&fragment, Some(transform * xform));
                }
                x += glyph.advance;
            }
        }
    }
}
