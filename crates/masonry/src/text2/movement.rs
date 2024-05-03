// Copyright 2018 the Xilem Authors and the Druid Authors
// SPDX-License-Identifier: Apache-2.0

//! Text editing movements.

use std::ops::Range;

use unicode_segmentation::UnicodeSegmentation;

use crate::kurbo::Point;

use super::{layout::TextLayout, Selectable, TextStorage};

/// Compute the result of a [`Movement`] on a [`Selection`].
///
/// returns a new selection representing the state after the movement.
///
/// If `modify` is true, only the 'active' edge (the `end`) of the selection
/// should be changed; this is the case when the user moves with the shift
/// key pressed.
pub fn movement<T: Selectable + TextStorage>(
    m: Movement,
    s: Selection,
    layout: &TextLayout<T>,
    modify: bool,
) -> Selection {
    if layout.needs_rebuild() {
        debug_panic!("movement() called before layout rebuild");
        return s;
    }
    let text = layout.text();
    let parley_layout = layout.layout();

    let writing_direction = || {
        if layout
            .cursor_for_text_position(s.active, s.active_affinity)
            .is_rtl
        {
            WritingDirection::RightToLeft
        } else {
            WritingDirection::LeftToRight
        }
    };

    let (offset, h_pos) = match m {
        Movement::Grapheme(d) => {
            let direction = writing_direction();
            if d.is_upstream_for_direction(direction) {
                if s.is_caret() || modify {
                    text.prev_grapheme_offset(s.active)
                        .map(|off| (off, None))
                        .unwrap_or((0, s.h_pos))
                } else {
                    (s.min(), None)
                }
            } else {
                if s.is_caret() || modify {
                    text.next_grapheme_offset(s.active)
                        .map(|off| (off, None))
                        .unwrap_or((s.active, s.h_pos))
                } else {
                    (s.max(), None)
                }
            }
        }
        Movement::Vertical(VerticalMovement::LineUp) => {
            let cur_pos = layout.cursor_for_text_position(s.active, s.active_affinity);
            let h_pos = s.h_pos.unwrap_or(cur_pos.advance);
            if cur_pos.path.line_index == 0 {
                (0, Some(h_pos))
            } else {
                let lm = cur_pos.path.line(&parley_layout).unwrap();
                let point_above = Point::new(h_pos, cur_pos.point.y - lm.height);
                let up_pos = layout.hit_test_point(point_above);
                if up_pos.is_inside {
                    (up_pos.idx, Some(h_pos))
                } else {
                    // because we can't specify affinity, moving up when h_pos
                    // is wider than both the current line and the previous line
                    // can result in a cursor position at the visual start of the
                    // current line; so we handle this as a special-case.
                    let lm_prev = layout.line_metric(cur_pos.line.saturating_sub(1)).unwrap();
                    let up_pos = lm_prev.end_offset - lm_prev.trailing_whitespace;
                    (up_pos, Some(h_pos))
                }
            }
        }
        Movement::Vertical(VerticalMovement::LineDown) => {
            let cur_pos = layout.hit_test_text_position(s.active);
            let h_pos = s.h_pos.unwrap_or(cur_pos.point.x);
            if cur_pos.line == layout.line_count() - 1 {
                (text.len(), Some(h_pos))
            } else {
                let lm = layout.line_metric(cur_pos.line).unwrap();
                // may not work correctly for point sizes below 1.0
                let y_below = lm.y_offset + lm.height + 1.0;
                let point_below = Point::new(h_pos, y_below);
                let up_pos = layout.hit_test_point(point_below);
                (up_pos.idx, Some(point_below.x))
            }
        }
        Movement::Vertical(VerticalMovement::DocumentStart) => (0, None),
        Movement::Vertical(VerticalMovement::DocumentEnd) => (text.len(), None),

        Movement::ParagraphStart => (text.preceding_line_break(s.active), None),
        Movement::ParagraphEnd => (text.next_line_break(s.active), None),

        Movement::Line(d) => {
            let hit = layout.hit_test_text_position(s.active);
            let lm = layout.line_metric(hit.line).unwrap();
            let offset = if d.is_upstream_for_direction(writing_direction) {
                lm.start_offset
            } else {
                lm.end_offset - lm.trailing_whitespace
            };
            (offset, None)
        }
        Movement::Word(d) => {
            if d.is_upstream_for_direction(writing_direction()) {
                let offset = if s.is_caret() || modify {
                    text.prev_word_offset(s.active).unwrap_or(0)
                } else {
                    s.min()
                };
                (offset, None)
            } else {
                let offset = if s.is_caret() || modify {
                    text.next_word_offset(s.active).unwrap_or(s.active)
                } else {
                    s.max()
                };
                (offset, None)
            }
        }

        // These two are not handled; they require knowledge of the size
        // of the viewport.
        Movement::Vertical(VerticalMovement::PageDown)
        | Movement::Vertical(VerticalMovement::PageUp) => (s.active, s.h_pos),
        other => {
            tracing::warn!("unhandled movement {:?}", other);
            (s.anchor, s.h_pos)
        }
    };

    let start = if modify { s.anchor } else { offset };
    Selection::new(start, offset).with_h_pos(h_pos)
}

/// Indicates a movement that transforms a particular text position in a
/// document.
///
/// These movements transform only single indices — not selections.
///
/// You'll note that a lot of these operations are idempotent, but you can get
/// around this by first sending a `Grapheme` movement.  If for instance, you
/// want a `ParagraphStart` that is not idempotent, you can first send
/// `Movement::Grapheme(Direction::Upstream)`, and then follow it with
/// `ParagraphStart`.
#[non_exhaustive]
#[derive(Clone, Copy, Debug, PartialEq, Eq, Hash)]
pub enum Movement {
    /// A movement that stops when it reaches an extended grapheme cluster boundary.
    ///
    /// This movement is achieved on most systems by pressing the left and right
    /// arrow keys.  For more information on grapheme clusters, see
    /// [Unicode Text Segmentation](https://unicode.org/reports/tr29/#Grapheme_Cluster_Boundaries).
    Grapheme(Direction),
    /// A movement that stops when it reaches a word boundary.
    ///
    /// This movement is achieved on most systems by pressing the left and right
    /// arrow keys while holding control. For more information on words, see
    /// [Unicode Text Segmentation](https://unicode.org/reports/tr29/#Word_Boundaries).
    Word(Direction),
    /// A movement that stops when it reaches a soft line break.
    ///
    /// This movement is achieved on macOS by pressing the left and right arrow
    /// keys while holding command.  `Line` should be idempotent: if the
    /// position is already at the end of a soft-wrapped line, this movement
    /// should never push it onto another soft-wrapped line.
    ///
    /// In order to implement this properly, your text positions should remember
    /// their affinity.
    Line(Direction),
    /// An upstream movement that stops when it reaches a hard line break.
    ///
    /// `ParagraphStart` should be idempotent: if the position is already at the
    /// start of a hard-wrapped line, this movement should never push it onto
    /// the previous line.
    ParagraphStart,
    /// A downstream movement that stops when it reaches a hard line break.
    ///
    /// `ParagraphEnd` should be idempotent: if the position is already at the
    /// end of a hard-wrapped line, this movement should never push it onto the
    /// next line.
    ParagraphEnd,
    /// A vertical movement, see `VerticalMovement` for more details.
    Vertical(VerticalMovement),
}

/// Indicates a horizontal direction in the text.
#[derive(Clone, Copy, Debug, PartialEq, Eq, Hash)]
pub enum Direction {
    /// The direction visually to the left.
    ///
    /// This may be byte-wise forwards or backwards in the document, depending
    /// on the text direction around the position being moved.
    Left,
    /// The direction visually to the right.
    ///
    /// This may be byte-wise forwards or backwards in the document, depending
    /// on the text direction around the position being moved.
    Right,
    /// Byte-wise backwards in the document.
    ///
    /// In a left-to-right context, this value is the same as `Left`.
    Upstream,
    /// Byte-wise forwards in the document.
    ///
    /// In a left-to-right context, this value is the same as `Right`.
    Downstream,
}

impl Direction {
    /// Returns `true` if this direction is byte-wise backwards for
    /// the provided [`WritingDirection`].
    ///
    /// The provided direction *must not be* `WritingDirection::Natural`.
    pub fn is_upstream_for_direction(self, direction: WritingDirection) -> bool {
        // assert!(
        //     !matches!(direction, WritingDirection::Natural),
        //     "writing direction must be resolved"
        // );
        match self {
            Direction::Upstream => true,
            Direction::Downstream => false,
            Direction::Left => matches!(direction, WritingDirection::LeftToRight),
            Direction::Right => matches!(direction, WritingDirection::RightToLeft),
        }
    }
}

/// Distinguishes between two visually distinct locations with the same byte
/// index.
///
/// Sometimes, a byte location in a document has two visual locations. For
/// example, the end of a soft-wrapped line and the start of the subsequent line
/// have different visual locations (and we want to be able to place an input
/// caret in either place!) but the same byte-wise location. This also shows up
/// in bidirectional text contexts. Affinity allows us to disambiguate between
/// these two visual locations.
///
/// Note that in scenarios where soft line breaks interact with bidi text, this gets
/// more complicated.
#[derive(Copy, Clone, Debug, Hash, PartialEq)]
pub enum Affinity {
    /// The position which has an apparent position "earlier" in the text.
    /// For soft line breaks, this is the position at the end of the first line.
    ///
    /// For positions in-between bidi contexts, this is the position which is
    /// related to the "outgoing" text section. E.g. for the string "abcDEF" (rendered `abcFED`),
    /// with the cursor at "abc|DEF" with upstream affinity, the cursor would be rendered at the
    /// position `abc|DEF`
    Upstream,
    /// The position which has a higher apparent position in the text.
    /// For soft line breaks, this is the position at the beginning of the second line.
    ///
    /// For positions in-between bidi contexts, this is the position which is
    /// related to the "incoming" text section. E.g. for the string "abcDEF" (rendered `abcFED`),
    /// with the cursor at "abc|DEF" with downstream affinity, the cursor would be rendered at the
    /// position `abcDEF|`
    Downstream,
}

impl Affinity {
    /// Convert into the `parley` form of "leading"
    pub fn is_leading(&self) -> bool {
        match self {
            Affinity::Upstream => false,
            Affinity::Downstream => true,
        }
    }
}

/// Indicates a horizontal direction for writing text.
#[derive(Clone, Copy, Debug, PartialEq, Eq, Hash)]
pub enum WritingDirection {
    LeftToRight,
    RightToLeft,
    // /// Indicates writing direction should be automatically detected based on
    // /// the text contents.
    // See also `is_upstream_for_direction` if adding back in
    // Natural,
}

/// Indicates a vertical movement in a text document.
#[non_exhaustive]
#[derive(Clone, Copy, Debug, PartialEq, Eq, Hash)]
pub enum VerticalMovement {
    LineUp,
    LineDown,
    PageUp,
    PageDown,
    DocumentStart,
    DocumentEnd,
}

/// Given a position in some text, return the containing word boundaries.
///
/// The returned range may not necessary be a 'word'; for instance it could be
/// the sequence of whitespace between two words.
///
/// If the position is on a word boundary, that will be considered the start
/// of the range.
///
/// This uses Unicode word boundaries, as defined in [UAX#29].
///
/// [UAX#29]: http://www.unicode.org/reports/tr29/
pub(crate) fn word_range_for_pos(text: &str, pos: usize) -> Range<usize> {
    text.split_word_bound_indices()
        .map(|(ix, word)| ix..(ix + word.len()))
        .find(|range| range.contains(&pos))
        .unwrap_or(pos..pos)
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn word_range_simple() {
        assert_eq!(word_range_for_pos("hello world", 3), 0..5);
        assert_eq!(word_range_for_pos("hello world", 8), 6..11);
    }

    #[test]
    fn word_range_whitespace() {
        assert_eq!(word_range_for_pos("hello world", 5), 5..6);
    }

    #[test]
    fn word_range_rtl() {
        let rtl = "مرحبا بالعالم";
        assert_eq!(word_range_for_pos(rtl, 5), 0..10);
        assert_eq!(word_range_for_pos(rtl, 16), 11..25);
        assert_eq!(word_range_for_pos(rtl, 10), 10..11);
    }

    #[test]
    fn word_range_mixed() {
        let mixed = "hello مرحبا بالعالم world";
        assert_eq!(word_range_for_pos(mixed, 3), 0..5);
        assert_eq!(word_range_for_pos(mixed, 8), 6..16);
        assert_eq!(word_range_for_pos(mixed, 19), 17..31);
        assert_eq!(word_range_for_pos(mixed, 36), 32..37);
    }
}
