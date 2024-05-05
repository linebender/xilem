// Copyright 2020 the Xilem Authors and the Druid Authors
// SPDX-License-Identifier: Apache-2.0

//! Helper functions for working with text in Masonry.

use kurbo::Rect;
use parley::Layout;
use vello::{kurbo::Affine, peniko::Fill, Scene};

use crate::{text2::TextBrush, WidgetId};

/// A reference counted string slice.
///
/// This is a data-friendly way to represent strings in Masonry. Unlike `String`
/// it cannot be mutated, but unlike `String` it can be cheaply cloned.
pub type ArcStr = std::sync::Arc<str>;

/// A type we use to keep track of which widgets are responsible for which
/// ime sessions.
#[derive(Clone, Debug)]
#[allow(unused)]
pub(crate) struct TextFieldRegistration {
    pub widget_id: WidgetId,
}

// Copy-pasted from druid_shell
/// An event representing an application-initiated change in [`InputHandler`]
/// state.
///
/// When we change state that may have previously been retrieved from an
/// [`InputHandler`], we notify the platform so that it can invalidate any
/// data if necessary.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
#[non_exhaustive]
pub enum ImeChangeSignal {
    /// Indicates the value returned by `InputHandler::selection` may have changed.
    SelectionChanged,

    /// Indicates the values returned by one or more of these methods may have changed:
    /// - `InputHandler::hit_test_point`
    /// - `InputHandler::line_range`
    /// - `InputHandler::bounding_box`
    /// - `InputHandler::slice_bounding_box`
    LayoutChanged,

    /// Indicates any value returned from any `InputHandler` method may have changed.
    Reset,
}

/// A function that renders laid out glyphs to a [Scene].
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
            let text_brush = match &style.brush {
                TextBrush::Normal(text_brush) => text_brush,
                TextBrush::Highlight { text, fill } => {
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

                    text
                }
            };
            scratch_scene
                .draw_glyphs(font)
                .brush(text_brush)
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
                        vello::glyph::Glyph {
                            id: glyph.id as _,
                            x: gx,
                            y: gy,
                        }
                    }),
                );
        }
    }
    scene.append(scratch_scene, None);
}
