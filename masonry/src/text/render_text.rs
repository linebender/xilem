// Copyright 2020 the Xilem Authors and the Druid Authors
// SPDX-License-Identifier: Apache-2.0

//! Helper functions for working with text in Masonry.

use parley::Layout;
use vello::kurbo::{Affine, Line, Rect, Stroke};
use vello::peniko::Fill;
use vello::Scene;

use crate::text::TextBrush;

/// A function that renders laid out glyphs to a [`Scene`].
pub fn render_text(
    scene: &mut Scene,
    scratch_scene: &mut Scene,
    transform: Affine,
    layout: &Layout<TextBrush>,
) {
    scratch_scene.reset();
    for line in layout.lines() {
        let metrics = &line.metrics();
        for glyph_run in line.glyph_runs() {
            let mut x = glyph_run.offset();
            let y = glyph_run.baseline();
            let run = glyph_run.run();
            let font = run.font();
            let font_size = run.font_size();
            let synthesis = run.synthesis();
            let glyph_xform = synthesis
                .skew()
                .map(|angle| Affine::skew(angle.to_radians().tan() as f64, 0.0));
            let style = glyph_run.style();
            let coords = run
                .normalized_coords()
                .iter()
                .map(|coord| vello::skrifa::instance::NormalizedCoord::from_bits(*coord))
                .collect::<Vec<_>>();
            let (text_brush, hinting) = match &style.brush {
                TextBrush::Normal(text_brush, hinting) => (text_brush, hinting),
                TextBrush::Highlight {
                    text,
                    fill,
                    hinting,
                } => {
                    scene.fill(
                        Fill::EvenOdd,
                        transform,
                        fill,
                        None,
                        &Rect::from_origin_size(
                            (
                                glyph_run.offset() as f64,
                                // The y coordinate is on the baseline. We want to draw from the top of the line
                                // (Note that we are in a y-down coordinate system)
                                (y - metrics.ascent - metrics.leading) as f64,
                            ),
                            (glyph_run.advance() as f64, metrics.size() as f64),
                        ),
                    );

                    (text, hinting)
                }
            };
            scratch_scene
                .draw_glyphs(font)
                .brush(text_brush)
                .hint(hinting.should_hint())
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
            if let Some(underline) = &style.underline {
                let underline_brush = match &underline.brush {
                    // Underlines aren't hinted
                    TextBrush::Normal(text, _) => text,
                    // It doesn't make sense for an underline to have a highlight colour, so we
                    // just use the text colour for the colour
                    TextBrush::Highlight { text, .. } => text,
                };
                let run_metrics = glyph_run.run().metrics();
                let offset = match underline.offset {
                    Some(offset) => offset,
                    None => run_metrics.underline_offset,
                };
                let width = match underline.size {
                    Some(size) => size,
                    None => run_metrics.underline_size,
                };
                // The `offset` is the distance from the baseline to the *top* of the underline
                // so we move the line down by half the width
                // Remember that we are using a y-down coordinate system
                let y = glyph_run.baseline() - offset + width / 2.;

                let line = Line::new(
                    (glyph_run.offset() as f64, y as f64),
                    ((glyph_run.offset() + glyph_run.advance()) as f64, y as f64),
                );
                scratch_scene.stroke(
                    &Stroke::new(width.into()),
                    transform,
                    underline_brush,
                    None,
                    &line,
                );
            }
            if let Some(strikethrough) = &style.strikethrough {
                let strikethrough_brush = match &strikethrough.brush {
                    // Strikethroughs aren't hinted
                    TextBrush::Normal(text, _) => text,
                    // It doesn't make sense for an underline to have a highlight colour, so we
                    // just use the text colour for the colour
                    TextBrush::Highlight { text, .. } => text,
                };
                let run_metrics = glyph_run.run().metrics();
                let offset = match strikethrough.offset {
                    Some(offset) => offset,
                    None => run_metrics.strikethrough_offset,
                };
                let width = match strikethrough.size {
                    Some(size) => size,
                    None => run_metrics.strikethrough_size,
                };
                // The `offset` is the distance from the baseline to the *top* of the strikethrough
                // so we move the line down by half the width
                // Remember that we are using a y-down coordinate system
                let y = glyph_run.baseline() - offset + width / 2.;

                let line = Line::new(
                    (glyph_run.offset() as f64, y as f64),
                    ((glyph_run.offset() + glyph_run.advance()) as f64, y as f64),
                );
                scratch_scene.stroke(
                    &Stroke::new(width.into()),
                    transform,
                    strikethrough_brush,
                    None,
                    &line,
                );
            }
        }
    }
    scene.append(scratch_scene, None);
}
