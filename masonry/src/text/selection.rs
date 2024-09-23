// Copyright 2018 the Xilem Authors and the Druid Authors and the Glazier Authors
// SPDX-License-Identifier: Apache-2.0

//! Traits for text editing and a basic String implementation.

use std::borrow::Cow;
use std::ops::{Deref, DerefMut, Range};

use parley::context::RangedBuilder;
use parley::{FontContext, LayoutContext};
use tracing::debug;
use unicode_segmentation::{GraphemeCursor, UnicodeSegmentation};
use vello::kurbo::{Affine, Line, Point, Stroke};
use vello::peniko::{Brush, Color};
use vello::Scene;
use winit::keyboard::NamedKey;

use crate::event::{PointerButton, PointerState};
use crate::{Handled, TextEvent};

use super::{TextBrush, TextLayout};

pub struct TextWithSelection<T: Selectable> {
    pub layout: TextLayout<T>,
    /// The current selection within this widget
    // TODO: Allow multiple selections (i.e. by holding down control)
    pub selection: Option<Selection>,
    highlight_brush: TextBrush,
    needs_selection_update: bool,
    selecting_with_mouse: bool,
    // TODO: Cache cursor line, selection boxes
    cursor_line: Option<Line>,
}

impl<T: Selectable> TextWithSelection<T> {
    pub fn new(text: T, text_size: f32) -> Self {
        Self {
            layout: TextLayout::new(text, text_size),
            selection: None,
            needs_selection_update: false,
            selecting_with_mouse: false,
            cursor_line: None,
            highlight_brush: TextBrush::Highlight {
                text: Color::WHITE.into(),
                fill: Color::LIGHT_BLUE.into(),
                hinting: Default::default(),
            },
        }
    }

    pub fn set_text(&mut self, text: T) {
        self.selection = None;
        self.needs_selection_update = true;
        self.layout.set_text(text);
    }

    pub fn needs_rebuild(&self) -> bool {
        self.layout.needs_rebuild() || self.needs_selection_update
    }

    pub fn pointer_down(
        &mut self,
        origin: Point,
        state: &PointerState,
        button: PointerButton,
    ) -> bool {
        // TODO: work out which button is the primary button?
        if button == PointerButton::Primary {
            self.selecting_with_mouse = true;
            self.needs_selection_update = true;
            // TODO: Much of this juggling seems unnecessary
            let position = Point::new(state.position.x, state.position.y) - origin;
            let position = self
                .layout
                .cursor_for_point(Point::new(position.x, position.y));
            tracing::warn!("Got cursor point without getting affinity");
            if state.mods.state().shift_key() {
                if let Some(selection) = self.selection.as_mut() {
                    selection.active = position.insert_point;
                    selection.active_affinity = Affinity::Downstream;
                    return true;
                }
            }
            self.selection = Some(Selection::caret(
                position.insert_point,
                Affinity::Downstream,
            ));
            true
        } else {
            false
        }
    }

    pub fn pointer_up(&mut self, _origin: Point, _state: &PointerState, button: PointerButton) {
        if button == PointerButton::Primary {
            self.selecting_with_mouse = false;
        }
    }

    pub fn pointer_move(&mut self, origin: Point, state: &PointerState) -> bool {
        if self.selecting_with_mouse {
            self.needs_selection_update = true;
            let position = Point::new(state.position.x, state.position.y) - origin;
            let position = self
                .layout
                .cursor_for_point(Point::new(position.x, position.y));
            tracing::warn!("Got cursor point without getting affinity");
            if let Some(selection) = self.selection.as_mut() {
                selection.active = position.insert_point;
                selection.active_affinity = Affinity::Downstream;
            } else {
                debug_panic!("No selection set whilst still dragging");
            }
            true
        } else {
            false
        }
    }

    pub fn text_event(&mut self, event: &TextEvent) -> Handled {
        match event {
            TextEvent::KeyboardKey(key, mods) if key.state.is_pressed() => {
                match shortcut_key(key) {
                    winit::keyboard::Key::Named(NamedKey::ArrowLeft) => {
                        if mods.shift_key() {
                        } else {
                            let t = self.text();
                            if let Some(selection) = self.selection {
                                if mods.control_key() {
                                    let offset = t.prev_word_offset(selection.active).unwrap_or(0);
                                    self.selection =
                                        Some(Selection::caret(offset, Affinity::Downstream));
                                } else {
                                    let offset =
                                        t.prev_grapheme_offset(selection.active).unwrap_or(0);
                                    self.selection =
                                        Some(Selection::caret(offset, Affinity::Downstream));
                                };
                            }
                        }
                        Handled::Yes
                    }
                    winit::keyboard::Key::Named(NamedKey::ArrowRight) => {
                        if mods.shift_key() {
                            // TODO: Expand selection
                        } else {
                            let t = self.text();
                            if let Some(selection) = self.selection {
                                if mods.control_key() {
                                    if let Some(o) = t.next_word_offset(selection.active) {
                                        self.selection =
                                            Some(Selection::caret(o, Affinity::Upstream));
                                    }
                                } else if let Some(o) = t.next_grapheme_offset(selection.active) {
                                    self.selection = Some(Selection::caret(o, Affinity::Upstream));
                                };
                            }
                        }
                        Handled::Yes
                    }
                    winit::keyboard::Key::Named(_) => Handled::No,
                    winit::keyboard::Key::Character(chr) => match &*chr {
                        "a" if mods.control_key() || /* macOS, yes this is a hack */ mods.super_key() =>
                        {
                            self.selection =
                                Some(Selection::new(0, self.text().len(), Affinity::Downstream));
                            self.needs_selection_update = true;
                            Handled::Yes
                        }
                        "c" if mods.control_key() || mods.super_key() => {
                            let selection = self.selection.unwrap_or(Selection {
                                anchor: 0,
                                active: 0,
                                active_affinity: Affinity::Downstream,
                                h_pos: None,
                            });
                            // TODO: We know this is not the fullest model of copy-paste, and that we should work with the inner text
                            // e.g. to put HTML code if supported by the rich text kind
                            if let Some(text) = self.text().slice(selection.min()..selection.max())
                            {
                                debug!(r#"Copying "{text}""#);
                            } else {
                                debug_panic!("Had invalid selection");
                            }
                            Handled::Yes
                        }
                        _ => Handled::No,
                    },
                    winit::keyboard::Key::Unidentified(_) => Handled::No,
                    winit::keyboard::Key::Dead(_) => Handled::No,
                }
            }
            TextEvent::KeyboardKey(_, _) => Handled::No,
            TextEvent::Ime(_) => Handled::No,
            TextEvent::ModifierChange(_) => {
                // TODO: What does it mean to "handle" this change?
                Handled::No
            }
            TextEvent::FocusChange(_) => {
                // TODO: What does it mean to "handle" this change
                // TODO: Set our highlighting colour to a lighter blue if window unfocused
                Handled::No
            }
        }
    }

    /// Call when this widget becomes focused
    pub fn focus_gained(&mut self) {
        if self.selection.is_none() {
            // TODO - We need to have some "memory" of the text selected instead.
            self.selection = Some(Selection::caret(self.text().len(), Affinity::Downstream));
        }
        self.needs_selection_update = true;
    }

    /// Call when another widget becomes focused
    pub fn focus_lost(&mut self) {
        self.selection = None;
        self.selecting_with_mouse = false;
        self.needs_selection_update = true;
    }

    /// Rebuild the text layout.
    ///
    /// See also [`TextLayout::rebuild`] for more comprehensive docs.
    pub fn rebuild(
        &mut self,
        font_ctx: &mut FontContext,
        layout_ctx: &mut LayoutContext<TextBrush>,
    ) {
        self.rebuild_with_attributes(font_ctx, layout_ctx, |builder| builder);
    }

    // Intentionally aliases the method on `TextLayout`
    /// Rebuild the text layout, adding attributes to the builder.
    ///
    /// See also [`TextLayout::rebuild_with_attributes`] for more comprehensive docs.
    pub fn rebuild_with_attributes(
        &mut self,
        font_ctx: &mut FontContext,
        layout_ctx: &mut LayoutContext<TextBrush>,
        attributes: impl for<'b> FnOnce(
            RangedBuilder<'b, TextBrush, &'b str>,
        ) -> RangedBuilder<'b, TextBrush, &'b str>,
    ) {
        // In theory, we could be clever here and only rebuild the layout if the
        // selected range was previously or currently non-zero size (i.e. there is a selected range)
        if self.needs_selection_update || self.layout.needs_rebuild() {
            self.layout.invalidate();
            self.layout
                .rebuild_with_attributes(font_ctx, layout_ctx, |mut builder| {
                    if let Some(selection) = self.selection {
                        let range = selection.range();
                        if !range.is_empty() {
                            builder.push(
                                &parley::style::StyleProperty::Brush(self.highlight_brush.clone()),
                                range,
                            );
                        }
                    }
                    attributes(builder)
                });
            self.needs_selection_update = false;
        }
    }

    pub fn draw(&mut self, scene: &mut Scene, point: impl Into<Point>) {
        // TODO: Calculate the location for this in layout lazily?
        if let Some(selection) = self.selection {
            self.cursor_line = Some(self.layout.cursor_line_for_text_position(selection.active));
        } else {
            self.cursor_line = None;
        }
        let point: Point = point.into();
        if let Some(line) = self.cursor_line {
            scene.stroke(
                &Stroke::new(2.),
                Affine::translate((point.x, point.y)),
                &Brush::Solid(Color::WHITE),
                None,
                &line,
            );
        }
        self.layout.draw(scene, point);
    }
}

/// Get the key which should be used for shortcuts from the underlying event
///
/// `key_without_modifiers` is only available on some platforms
fn shortcut_key(key: &winit::event::KeyEvent) -> winit::keyboard::Key {
    #[cfg(not(any(target_os = "android", target_os = "ios")))]
    {
        use winit::platform::modifier_supplement::KeyEventExtModifierSupplement;
        key.key_without_modifiers()
    }
    #[cfg(any(target_os = "android", target_os = "ios"))]
    // We think it will be rare that users are using a physical keyboard with Android,
    // and so we don't really need to worry *too much* about the text selection shortcuts
    key.logical_key.clone()
}

impl<T: Selectable> Deref for TextWithSelection<T> {
    type Target = TextLayout<T>;

    fn deref(&self) -> &Self::Target {
        &self.layout
    }
}

// TODO: Being able to call `Self::Target::rebuild` (and `draw`) isn't great.
impl<T: Selectable> DerefMut for TextWithSelection<T> {
    fn deref_mut(&mut self) -> &mut Self::Target {
        &mut self.layout
    }
}

/// A range of selected text, or a caret.
///
/// A caret is the blinking vertical bar where text is to be inserted. We
/// represent it as a selection with zero length, where `anchor == active`.
/// Indices are always expressed in UTF-8 bytes, and must be between 0 and the
/// document length, inclusive.
///
/// As an example, if the input caret is at the start of the document `hello
/// world`, we would expect both `anchor` and `active` to be `0`. If the user
/// holds shift and presses the right arrow key five times, we would expect the
/// word `hello` to be selected, the `anchor` to still be `0`, and the `active`
/// to now be `5`.
#[derive(Clone, Copy, Debug, PartialEq)]
#[non_exhaustive]
pub struct Selection {
    /// The 'anchor' end of the selection.
    ///
    /// This is the end of the selection that stays unchanged while holding
    /// shift and pressing the arrow keys.
    // TODO: Is usize the right type for these? Is it plausible to be dealing with a
    // more than 4gb file on a 32 bit machine?
    pub anchor: usize,
    /// The 'active' end of the selection.
    ///
    /// This is the end of the selection that moves while holding shift and
    /// pressing the arrow keys.
    pub active: usize,
    /// The affinity of the `active` side of the cursor
    ///
    /// The affinity of `anchor` is entirely based on the affinity of active:
    /// 1) If `active` is Upstream
    pub active_affinity: Affinity,
    /// The saved horizontal position, during vertical movement.
    ///
    /// This should not be set by the IME; it will be tracked and handled by
    /// the text field.
    pub h_pos: Option<f32>,
}

#[allow(clippy::len_without_is_empty)]
impl Selection {
    /// Create a new `Selection` with the provided `anchor` and `active` positions.
    ///
    /// Both positions refer to UTF-8 byte indices in some text.
    ///
    /// If your selection is a caret, you can use [`Selection::caret`] instead.
    pub fn new(anchor: usize, active: usize, active_affinity: Affinity) -> Selection {
        Selection {
            anchor,
            active,
            h_pos: None,
            active_affinity,
        }
    }

    /// Create a new caret (zero-length selection) at the provided UTF-8 byte index.
    ///
    /// `index` must be a grapheme cluster boundary.
    pub fn caret(index: usize, affinity: Affinity) -> Selection {
        Selection {
            anchor: index,
            active: index,
            h_pos: None,
            active_affinity: affinity,
        }
    }

    /// Construct a new selection from this selection, with the provided `h_pos`.
    ///
    /// # Note
    ///
    /// `h_pos` is used to track the *pixel* location of the cursor when moving
    /// vertically; lines may have available cursor positions at different
    /// positions, and arrowing down and then back up should always result
    /// in a cursor at the original starting location; doing this correctly
    /// requires tracking this state.
    ///
    /// You *probably* don't need to use this, unless you are implementing a new
    /// text field, or otherwise implementing vertical cursor motion, in which
    /// case you will want to set this during vertical motion if it is not
    /// already set.
    pub fn with_h_pos(mut self, h_pos: Option<f32>) -> Self {
        self.h_pos = h_pos;
        self
    }

    /// Create a new selection that is guaranteed to be valid for the provided
    /// text.
    #[must_use = "constrained constructs a new Selection"]
    pub fn constrained(mut self, s: &str) -> Self {
        let s_len = s.len();
        self.anchor = self.anchor.min(s_len);
        self.active = self.active.min(s_len);
        while !s.is_char_boundary(self.anchor) {
            self.anchor += 1;
        }
        while !s.is_char_boundary(self.active) {
            self.active += 1;
        }
        self
    }

    /// Return the position of the upstream end of the selection.
    ///
    /// This is end with the lesser byte index.
    ///
    /// Because of bidirectional text, this is not necessarily "left".
    pub fn min(&self) -> usize {
        usize::min(self.anchor, self.active)
    }

    /// Return the position of the downstream end of the selection.
    ///
    /// This is the end with the greater byte index.
    ///
    /// Because of bidirectional text, this is not necessarily "right".
    pub fn max(&self) -> usize {
        usize::max(self.anchor, self.active)
    }

    /// The sequential range of the document represented by this selection.
    ///
    /// This is the range that would be replaced if text were inserted at this
    /// selection.
    pub fn range(&self) -> Range<usize> {
        self.min()..self.max()
    }

    /// The length, in bytes of the selected region.
    ///
    /// If the selection is a caret, this is `0`.
    pub fn len(&self) -> usize {
        if self.anchor > self.active {
            self.anchor - self.active
        } else {
            self.active - self.anchor
        }
    }

    /// Returns `true` if the selection's length is `0`.
    pub fn is_caret(&self) -> bool {
        self.len() == 0
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
///
/// This also has an impact on rich text editing.
/// For example, if the cursor is in a region like `a|1`, where `a` is bold and `1` is not.
/// When editing, if we came from the start of the string, we should assume that the next
/// character will be bold, from the right italic.
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

/// Text which can have internal selections
pub trait Selectable: Sized + AsRef<str> + Eq {
    /// Get slice of text at range.
    fn slice(&self, range: Range<usize>) -> Option<Cow<str>>;

    /// Get length of text (in bytes).
    fn len(&self) -> usize;

    /// Get the previous word offset from the given offset, if it exists.
    fn prev_word_offset(&self, offset: usize) -> Option<usize>;

    /// Get the next word offset from the given offset, if it exists.
    fn next_word_offset(&self, offset: usize) -> Option<usize>;

    /// Get the next grapheme offset from the given offset, if it exists.
    fn prev_grapheme_offset(&self, offset: usize) -> Option<usize>;

    /// Get the next grapheme offset from the given offset, if it exists.
    fn next_grapheme_offset(&self, offset: usize) -> Option<usize>;

    /// Get the previous codepoint offset from the given offset, if it exists.
    fn prev_codepoint_offset(&self, offset: usize) -> Option<usize>;

    /// Get the next codepoint offset from the given offset, if it exists.
    fn next_codepoint_offset(&self, offset: usize) -> Option<usize>;

    /// Get the preceding line break offset from the given offset
    fn preceding_line_break(&self, offset: usize) -> usize;

    /// Get the next line break offset from the given offset
    fn next_line_break(&self, offset: usize) -> usize;

    /// Returns `true` if this text has 0 length.
    fn is_empty(&self) -> bool;
}

impl<Str: AsRef<str> + Eq> Selectable for Str {
    fn slice(&self, range: Range<usize>) -> Option<Cow<str>> {
        self.as_ref().get(range).map(Cow::from)
    }

    fn len(&self) -> usize {
        self.as_ref().len()
    }

    fn prev_grapheme_offset(&self, from: usize) -> Option<usize> {
        let mut c = GraphemeCursor::new(from, self.len(), true);
        c.prev_boundary(self.as_ref(), 0).unwrap()
    }

    fn next_grapheme_offset(&self, from: usize) -> Option<usize> {
        let mut c = GraphemeCursor::new(from, self.len(), true);
        c.next_boundary(self.as_ref(), 0).unwrap()
    }

    fn prev_codepoint_offset(&self, from: usize) -> Option<usize> {
        let mut c = StringCursor::new(self.as_ref(), from).unwrap();
        c.prev()
    }

    fn next_codepoint_offset(&self, from: usize) -> Option<usize> {
        let mut c = StringCursor::new(self.as_ref(), from).unwrap();
        if c.next().is_some() {
            Some(c.pos())
        } else {
            None
        }
    }

    fn prev_word_offset(&self, from: usize) -> Option<usize> {
        let mut offset = from;
        let mut passed_alphanumeric = false;
        for prev_grapheme in self.as_ref().get(0..from)?.graphemes(true).rev() {
            let is_alphanumeric = prev_grapheme.chars().next()?.is_alphanumeric();
            if is_alphanumeric {
                passed_alphanumeric = true;
            } else if passed_alphanumeric {
                return Some(offset);
            }
            offset -= prev_grapheme.len();
        }
        None
    }

    fn next_word_offset(&self, from: usize) -> Option<usize> {
        let mut offset = from;
        let mut passed_alphanumeric = false;
        for next_grapheme in self.as_ref().get(from..)?.graphemes(true) {
            let is_alphanumeric = next_grapheme.chars().next()?.is_alphanumeric();
            if is_alphanumeric {
                passed_alphanumeric = true;
            } else if passed_alphanumeric {
                return Some(offset);
            }
            offset += next_grapheme.len();
        }
        Some(self.len())
    }

    fn is_empty(&self) -> bool {
        self.as_ref().is_empty()
    }

    fn preceding_line_break(&self, from: usize) -> usize {
        let mut offset = from;

        for byte in self.as_ref().get(0..from).unwrap_or("").bytes().rev() {
            if byte == 0x0a {
                return offset;
            }
            offset -= 1;
        }

        0
    }

    fn next_line_break(&self, from: usize) -> usize {
        let mut offset = from;

        for char in self.as_ref().get(from..).unwrap_or("").bytes() {
            if char == 0x0a {
                return offset;
            }
            offset += 1;
        }

        self.len()
    }
}

/// A cursor type with helper methods for moving through strings.
#[derive(Debug)]
pub struct StringCursor<'a> {
    pub(crate) text: &'a str,
    pub(crate) position: usize,
}

impl<'a> StringCursor<'a> {
    pub fn new(text: &'a str, position: usize) -> Option<Self> {
        let res = Self { text, position };
        if res.is_boundary() {
            Some(res)
        } else {
            None
        }
    }
}

impl<'a> StringCursor<'a> {
    /// Set cursor position.
    pub(crate) fn set(&mut self, position: usize) {
        self.position = position;
    }

    /// Get cursor position.
    pub(crate) fn pos(&self) -> usize {
        self.position
    }

    /// Check if cursor position is at a codepoint boundary.
    pub(crate) fn is_boundary(&self) -> bool {
        self.text.is_char_boundary(self.position)
    }

    /// Move cursor to previous codepoint boundary, if it exists.
    /// Returns previous codepoint as usize offset, or `None` if this cursor was already at the first boundary.
    pub(crate) fn prev(&mut self) -> Option<usize> {
        let current_pos = self.pos();

        if current_pos == 0 {
            None
        } else {
            let mut len = 1;
            while !self.text.is_char_boundary(current_pos - len) {
                len += 1;
            }
            self.set(self.pos() - len);
            Some(self.pos())
        }
    }

    /// Move cursor to next codepoint boundary, if it exists.
    /// Returns current codepoint as usize offset.
    pub(crate) fn next(&mut self) -> Option<usize> {
        let current_pos = self.pos();

        if current_pos == self.text.len() {
            None
        } else {
            let b = self.text.as_bytes()[current_pos];
            self.set(current_pos + len_utf8_from_first_byte(b));
            Some(current_pos)
        }
    }

    /// Return codepoint preceding cursor offset and move cursor backward.
    pub(crate) fn prev_codepoint(&mut self) -> Option<char> {
        if let Some(prev) = self.prev() {
            self.text[prev..].chars().next()
        } else {
            None
        }
    }
}

pub fn len_utf8_from_first_byte(b: u8) -> usize {
    match b {
        b if b < 0x80 => 1,
        b if b < 0xe0 => 2,
        b if b < 0xf0 => 3,
        _ => 4,
    }
}

// --- MARK: TESTS ---
#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn prev_codepoint_offset() {
        let a = String::from("a\u{00A1}\u{4E00}\u{1F4A9}");
        assert_eq!(Some(6), a.prev_codepoint_offset(10));
        assert_eq!(Some(3), a.prev_codepoint_offset(6));
        assert_eq!(Some(1), a.prev_codepoint_offset(3));
        assert_eq!(Some(0), a.prev_codepoint_offset(1));
        assert_eq!(None, a.prev_codepoint_offset(0));
        let b = a.slice(1..10).unwrap().to_string();
        assert_eq!(Some(5), b.prev_codepoint_offset(9));
        assert_eq!(Some(2), b.prev_codepoint_offset(5));
        assert_eq!(Some(0), b.prev_codepoint_offset(2));
        assert_eq!(None, b.prev_codepoint_offset(0));
    }

    #[test]
    fn next_codepoint_offset() {
        let a = String::from("a\u{00A1}\u{4E00}\u{1F4A9}");
        assert_eq!(Some(10), a.next_codepoint_offset(6));
        assert_eq!(Some(6), a.next_codepoint_offset(3));
        assert_eq!(Some(3), a.next_codepoint_offset(1));
        assert_eq!(Some(1), a.next_codepoint_offset(0));
        assert_eq!(None, a.next_codepoint_offset(10));
        let b = a.slice(1..10).unwrap().to_string();
        assert_eq!(Some(9), b.next_codepoint_offset(5));
        assert_eq!(Some(5), b.next_codepoint_offset(2));
        assert_eq!(Some(2), b.next_codepoint_offset(0));
        assert_eq!(None, b.next_codepoint_offset(9));
    }

    #[test]
    fn prev_next() {
        let input = String::from("abc");
        let mut cursor = StringCursor::new(&input, 0).unwrap();
        assert_eq!(cursor.next(), Some(0));
        assert_eq!(cursor.next(), Some(1));
        assert_eq!(cursor.prev(), Some(1));
        assert_eq!(cursor.next(), Some(1));
        assert_eq!(cursor.next(), Some(2));
    }

    #[test]
    fn prev_grapheme_offset() {
        // A with ring, hangul, regional indicator "US"
        let a = String::from("A\u{030a}\u{110b}\u{1161}\u{1f1fa}\u{1f1f8}");
        assert_eq!(Some(9), a.prev_grapheme_offset(17));
        assert_eq!(Some(3), a.prev_grapheme_offset(9));
        assert_eq!(Some(0), a.prev_grapheme_offset(3));
        assert_eq!(None, a.prev_grapheme_offset(0));
    }

    #[test]
    fn next_grapheme_offset() {
        // A with ring, hangul, regional indicator "US"
        let a = String::from("A\u{030a}\u{110b}\u{1161}\u{1f1fa}\u{1f1f8}");
        assert_eq!(Some(3), a.next_grapheme_offset(0));
        assert_eq!(Some(9), a.next_grapheme_offset(3));
        assert_eq!(Some(17), a.next_grapheme_offset(9));
        assert_eq!(None, a.next_grapheme_offset(17));
    }

    #[test]
    fn prev_word_offset() {
        let a = String::from("Technically a word: ৬藏A\u{030a}\u{110b}\u{1161}");
        assert_eq!(Some(20), a.prev_word_offset(35));
        assert_eq!(Some(20), a.prev_word_offset(27));
        assert_eq!(Some(20), a.prev_word_offset(23));
        assert_eq!(Some(14), a.prev_word_offset(20));
        assert_eq!(Some(14), a.prev_word_offset(19));
        assert_eq!(Some(12), a.prev_word_offset(13));
        assert_eq!(None, a.prev_word_offset(12));
        assert_eq!(None, a.prev_word_offset(11));
        assert_eq!(None, a.prev_word_offset(0));
    }

    #[test]
    fn next_word_offset() {
        let a = String::from("Technically a word: ৬藏A\u{030a}\u{110b}\u{1161}");
        assert_eq!(Some(11), a.next_word_offset(0));
        assert_eq!(Some(11), a.next_word_offset(7));
        assert_eq!(Some(13), a.next_word_offset(11));
        assert_eq!(Some(18), a.next_word_offset(14));
        assert_eq!(Some(35), a.next_word_offset(18));
        assert_eq!(Some(35), a.next_word_offset(19));
        assert_eq!(Some(35), a.next_word_offset(20));
        assert_eq!(Some(35), a.next_word_offset(26));
        assert_eq!(Some(35), a.next_word_offset(35));
    }

    #[test]
    fn preceding_line_break() {
        let a = String::from("Technically\na word:\n ৬藏A\u{030a}\n\u{110b}\u{1161}");
        assert_eq!(0, a.preceding_line_break(0));
        assert_eq!(0, a.preceding_line_break(11));
        assert_eq!(12, a.preceding_line_break(12));
        assert_eq!(12, a.preceding_line_break(13));
        assert_eq!(20, a.preceding_line_break(21));
        assert_eq!(31, a.preceding_line_break(31));
        assert_eq!(31, a.preceding_line_break(34));

        let b = String::from("Technically a word: ৬藏A\u{030a}\u{110b}\u{1161}");
        assert_eq!(0, b.preceding_line_break(0));
        assert_eq!(0, b.preceding_line_break(11));
        assert_eq!(0, b.preceding_line_break(13));
        assert_eq!(0, b.preceding_line_break(21));
    }

    #[test]
    fn next_line_break() {
        let a = String::from("Technically\na word:\n ৬藏A\u{030a}\n\u{110b}\u{1161}");
        assert_eq!(11, a.next_line_break(0));
        assert_eq!(11, a.next_line_break(11));
        assert_eq!(19, a.next_line_break(13));
        assert_eq!(30, a.next_line_break(21));
        assert_eq!(a.len(), a.next_line_break(31));

        let b = String::from("Technically a word: ৬藏A\u{030a}\u{110b}\u{1161}");
        assert_eq!(b.len(), b.next_line_break(0));
        assert_eq!(b.len(), b.next_line_break(11));
        assert_eq!(b.len(), b.next_line_break(13));
        assert_eq!(b.len(), b.next_line_break(19));
    }
}
