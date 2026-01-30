// Copyright 2018 the Xilem Authors and the Druid Authors
// SPDX-License-Identifier: Apache-2.0

//! Support for text display and rendering
//!
//! There are three kinds of text commonly needed:
//!  1) Non interactive text (e.g. a button's label)
//!  2) Selectable text (e.g. a paragraph of content)
//!  3) Editable text (e.g. a search bar)
//!
//! All of these have the same set of global styling options, and can contain rich text

/// A reference counted string slice.
///
/// This is a data-friendly way to represent strings in Masonry. Unlike `String`
/// it cannot be mutated, but unlike `String` it can be cheaply cloned.
pub type ArcStr = std::sync::Arc<str>;

/// The Parley [`Brush`] used within Masonry.
///
/// This enables updating of brush details without performing relayouts;
/// the inner values are indexes into the `brushes` argument to [`render_text()`].
///
/// [`Brush`]: parley::Brush
#[derive(Clone, PartialEq, Default, Debug)]
pub struct BrushIndex(pub usize);

/// A style property specialised for use within Masonry.
pub type StyleProperty = parley::StyleProperty<'static, BrushIndex>;

/// A set of styles specialised for use within Masonry.
pub type StyleSet = parley::StyleSet<BrushIndex>;

use parley::{Layout, PositionedLayoutItem};
use vello::Scene;
use vello::kurbo::{Affine, Line, Stroke};
use vello::peniko::{Brush, Fill};

/// A function that renders laid out glyphs to a [`Scene`].
///
/// The `BrushIndex` values of the runs are indices into `brushes`.
pub fn render_text(
    scene: &mut Scene,
    transform: Affine,
    layout: &Layout<BrushIndex>,
    brushes: &[Brush],
    // TODO: Should this be part of `BrushIndex` (i.e. `brushes`)?
    hint: bool,
) {
    for line in layout.lines() {
        for item in line.items() {
            let PositionedLayoutItem::GlyphRun(glyph_run) = item else {
                continue;
            };
            let style = glyph_run.style();
            // We draw underlines under the text, then the strikethrough on top, following:
            // https://drafts.csswg.org/css-text-decor/#painting-order
            if let Some(underline) = &style.underline {
                let underline_brush = &brushes[underline.brush.0];
                let run_metrics = glyph_run.run().metrics();
                let offset = match underline.offset {
                    Some(offset) => offset,
                    None => run_metrics.underline_offset,
                };
                let width = match underline.size {
                    Some(size) => size,
                    None => run_metrics.underline_size,
                };
                // The `offset` is the distance from the baseline to the top of the underline
                // so we move the line down by half the width
                // Remember that we are using a y-down coordinate system
                // If there's a custom width, because this is an underline, we want the custom
                // width to go down from the default expectation
                let y = glyph_run.baseline() - offset + width / 2.;

                let line = Line::new(
                    (glyph_run.offset() as f64, y as f64),
                    ((glyph_run.offset() + glyph_run.advance()) as f64, y as f64),
                );
                scene.stroke(
                    &Stroke::new(width.into()),
                    transform,
                    underline_brush,
                    None,
                    &line,
                );
            }
            let mut x = glyph_run.offset();
            let y = glyph_run.baseline();
            let run = glyph_run.run();
            let font = run.font();
            let font_size = run.font_size();
            let synthesis = run.synthesis();
            let glyph_xform = synthesis
                .skew()
                .map(|angle| Affine::skew(angle.to_radians().tan() as f64, 0.0));
            let coords = run.normalized_coords();
            let brush = &brushes[style.brush.0];
            scene
                .draw_glyphs(font)
                .brush(brush)
                .hint(hint)
                .transform(transform)
                .glyph_transform(glyph_xform)
                .font_size(font_size)
                .normalized_coords(coords)
                .draw(
                    Fill::NonZero,
                    glyph_run.glyphs().map(|glyph| {
                        let gx = x + glyph.x;
                        let gy = y - glyph.y;
                        x += glyph.advance;
                        vello::Glyph {
                            id: glyph.id,
                            x: gx,
                            y: gy,
                        }
                    }),
                );

            if let Some(strikethrough) = &style.strikethrough {
                let strikethrough_brush = &brushes[strikethrough.brush.0];
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
                // so we calculate the middle y-position of the strikethrough based on the font's
                // standard strikethrough width.
                // Remember that we are using a y-down coordinate system
                let y = glyph_run.baseline() - offset + run_metrics.strikethrough_size / 2.;

                let line = Line::new(
                    (glyph_run.offset() as f64, y as f64),
                    ((glyph_run.offset() + glyph_run.advance()) as f64, y as f64),
                );
                scene.stroke(
                    &Stroke::new(width.into()),
                    transform,
                    strikethrough_brush,
                    None,
                    &line,
                );
            }
        }
    }
}
