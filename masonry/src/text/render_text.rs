// Copyright 2020 the Xilem Authors and the Druid Authors
// SPDX-License-Identifier: Apache-2.0

//! Helper functions for working with text in Masonry.

use parley::{Layout, PositionedLayoutItem};
use vello::kurbo::Affine;
use vello::peniko::{Brush, Fill};
use vello::Scene;

use super::BrushIndex;

/// A function that renders laid out glyphs to a [`Scene`].
pub fn render_text(
    scene: &mut Scene,
    transform: Affine,
    layout: &Layout<BrushIndex>,
    brushes: &[Brush],
) {
    for line in layout.lines() {
        for item in line.items() {
            let PositionedLayoutItem::GlyphRun(glyph_run) = item else {
                continue;
            };
            let mut x = glyph_run.offset();
            let y = glyph_run.baseline();
            let run = glyph_run.run();
            let font = run.font();
            let font_size = run.font_size();
            let synthesis = run.synthesis();
            let glyph_xform = synthesis
                .skew()
                .map(|angle| Affine::skew(angle.to_radians().tan() as f64, 0.0));
            let coords = run
                .normalized_coords()
                .iter()
                .map(|coord| vello::skrifa::instance::NormalizedCoord::from_bits(*coord))
                .collect::<Vec<_>>();
            let brush = &brushes[glyph_run.style().brush.0];
            scene
                .draw_glyphs(font)
                .brush(brush)
                .hint(true)
                .transform(transform)
                .glyph_transform(glyph_xform)
                .font_size(font_size)
                .normalized_coords(&coords)
                .draw(
                    Fill::NonZero,
                    glyph_run.glyphs().map(|glyph| {
                        let gx = x + glyph.x;
                        let gy = y - glyph.y;
                        x += glyph.advance;
                        vello::Glyph {
                            id: glyph.id as _,
                            x: gx,
                            y: gy,
                        }
                    }),
                );
        }
    }
}
