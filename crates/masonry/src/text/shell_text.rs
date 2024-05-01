// Copyright 2020 The Druid Authors.
//
// Licensed under the Apache License, Version 2.0 (the "License");
// you may not use this file except in compliance with the License.
// You may obtain a copy of the License at
//
//     http://www.apache.org/licenses/LICENSE-2.0
//
// Unless required by applicable law or agreed to in writing, software
// distributed under the License is distributed on an "AS IS" BASIS,
// WITHOUT WARRANTIES OR CONDITIONS OF ANY KIND, either express or implied.
// See the License for the specific language governing permissions and
// limitations under the License.

//! Types and functions for cross-platform text input.
//!
//! Text input is a notoriously difficult problem.  Unlike many other aspects of
//! user interfaces, text input can not be correctly modeled using discrete
//! events passed from the platform to the application. For example, many mobile
//! phones implement autocorrect: when the user presses the spacebar, the
//! platform peeks at the word directly behind the caret, and potentially
//! replaces it if it's misspelled. This means the platform needs to know the
//! contents of a text field. On other devices, the platform may need to draw an
//! emoji window under the caret, or look up the on-screen locations of letters
//! for crossing out with a stylus, both of which require fetching on-screen
//! coordinates from the application.
//!
//! This is all to say: text editing is a bidirectional conversation between the
//! application and the platform. The application, when the platform asks for
//! it, serves up text field coordinates and content.  The platform looks at
//! this information and combines it with input from keyboards (physical or
//! onscreen), voice, styluses, the user's language settings, and then sends
//! edit commands to the application.
//!
//! Many platforms have an additional complication: this input fusion often
//! happens in a different process from your application. If we don't
//! specifically account for this fact, we might get race conditions!  In the
//! autocorrect example, if I sloppily type "meoow" and press space, the
//! platform might issue edits to "delete backwards one word and insert meow".
//! However, if I concurrently click somewhere else in the document to move the
//! caret, this will replace some *other* word with "meow", and leave the
//! "meoow" disappointingly present. To mitigate this problem, we use locks,
//! represented by the `InputHandler` trait.
//!
//! ## Lifecycle of a Text Input
//!
//! 1. The user clicks a link or switches tabs, and the window content now
//!    contains a new text field.  The application registers this new field by
//!    calling `WindowHandle::add_text_field`, and gets a `TextFieldToken` that
//!    represents this new field.
//! 2. The user clicks on that text field, focusing it. The application lets the
//!    platform know by calling `WindowHandle::set_focused_text_field` with that
//!    field's `TextFieldToken`.
//! 3. The user presses a key on the keyboard. The platform first calls
//!    `WinHandler::key_down`. If this method returns `true`, the application
//!    has indicated the keypress was captured, and we skip the remaining steps.
//! 4. If `key_down` returned `false`, `druid-shell` forwards the key event to the
//!    platform's text input system
//! 5. The platform, in response to either this key event or some other user
//!    action, determines it's time for some text input. It calls
//!    `WinHandler::text_input` to acquire a lock on the text field's state.
//!    The application returns an `InputHandler` object corresponding to the
//!    requested text field. To prevent race conditions, your application may
//!    not make any changes
//!    to the text field's state until the platform drops the `InputHandler`.
//! 6. The platform calls various `InputHandler` methods to inspect and edit the
//!    text field's state. Later, usually within a few milliseconds, the
//!    platform drops the `InputHandler`, allowing the application to once again
//!    make changes to the text field's state. These commands might be "insert
//!    `q`" for a smartphone user tapping on their virtual keyboard, or
//!    "move the caret one word left" for a user pressing the left arrow key
//!    while holding control.
//! 7. Eventually, after many keypresses cause steps 3–6 to repeat, the user
//!    unfocuses the text field. The application indicates this to the platform
//!    by calling `set_focused_text_field`.  Note that even though focus has
//!    shifted away from our text field, the platform may still send edits to it
//!    by calling `WinHandler::text_input`.
//! 8. At some point, the user clicks a link or switches a tab, and the text
//!    field is no longer present in the window.  The application calls
//!    `WindowHandle::remove_text_field`, and the platform may no longer call
//!    `WinHandler::text_input` to make changes to it.
//!
//! The application also has a series of steps it follows if it wants to make
//! its own changes to the text field's state:
//!
//! 1. The application determines it would like to make a change to the text
//!    field; perhaps the user has scrolled and and the text field has changed
//!    its visible location on screen, or perhaps the user has clicked to move
//!    the caret to a new location.
//! 2. The application first checks to see if there's an outstanding
//!    `InputHandler` lock for this text field; if so, it waits until the last
//!    `InputHandler` is dropped before continuing.
//! 3. The application then makes the change to the text input. If the change
//!    would affect state visible from an `InputHandler`, the application must
//!    notify the platform via `WinHandler::update_text_field`.
//!
//! ## Supported Platforms
//!
//! Currently, `druid-shell` text input is fully implemented on macOS. Our goal
//! is to have full support for all `druid-shell` targets, but for now,
//! `InputHandler` calls are simulated from keypresses on other platforms, which
//! doesn't allow for IME input, dead keys, etc.

use std::ops::Range;

/// An event representing an application-initiated change in [`InputHandler`]
/// state.
///
/// When we change state that may have previously been retrieved from an
/// [`InputHandler`], we notify the platform so that it can invalidate any
/// data if necessary.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
#[non_exhaustive]
pub enum Event {
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
#[derive(Clone, Copy, Debug, Default, PartialEq)]
#[non_exhaustive]
pub struct Selection {
    /// The 'anchor' end of the selection.
    ///
    /// This is the end of the selection that stays unchanged while holding
    /// shift and pressing the arrow keys.
    pub anchor: usize,
    /// The 'active' end of the selection.
    ///
    /// This is the end of the selection that moves while holding shift and
    /// pressing the arrow keys.
    pub active: usize,
    /// The saved horizontal position, during vertical movement.
    ///
    /// This should not be set by the IME; it will be tracked and handled by
    /// the text field.
    pub h_pos: Option<f64>,
}

#[allow(clippy::len_without_is_empty)]
impl Selection {
    /// Create a new `Selection` with the provided `anchor` and `active` positions.
    ///
    /// Both positions refer to UTF-8 byte indices in some text.
    ///
    /// If your selection is a caret, you can use [`Selection::caret`] instead.
    pub fn new(anchor: usize, active: usize) -> Selection {
        Selection {
            anchor,
            active,
            h_pos: None,
        }
    }

    /// Create a new caret (zero-length selection) at the provided UTF-8 byte index.
    ///
    /// `index` must be a grapheme cluster boundary.
    pub fn caret(index: usize) -> Selection {
        Selection {
            anchor: index,
            active: index,
            h_pos: None,
        }
    }

    /// Construct a new selection from this selection, with the provided h_pos.
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
    pub fn with_h_pos(mut self, h_pos: Option<f64>) -> Self {
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
        assert!(
            !matches!(direction, WritingDirection::Natural),
            "writing direction must be resolved"
        );
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
pub enum Affinity {
    Upstream,
    Downstream,
}

/// Indicates a horizontal direction for writing text.
#[derive(Clone, Copy, Debug, PartialEq, Eq, Hash)]
pub enum WritingDirection {
    LeftToRight,
    RightToLeft,
    /// Indicates writing direction should be automatically detected based on
    /// the text contents.
    Natural,
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

/// A special text editing command sent from the platform to the application.
#[non_exhaustive]
#[derive(Clone, Copy, Debug, PartialEq, Eq, Hash)]
pub enum Action {
    /// Moves the selection.
    ///
    /// Before moving, if the active and the anchor of the selection are not at
    /// the same position (it's a non-caret selection), then:
    ///
    /// 1. First set both active and anchor to the same position: the
    ///    selection's upstream index if `Movement` is an upstream movement, or
    ///    the downstream index if `Movement` is a downstream movement.
    ///
    /// 2. If `Movement` is `Grapheme`, then stop. Otherwise, apply the
    ///    `Movement` as per the usual rules.
    Move(Movement),

    /// Moves just the selection's active edge.
    ///
    /// Equivalent to holding shift while performing movements or clicks on most
    /// operating systems.
    MoveSelecting(Movement),

    /// Select the entire document.
    SelectAll,

    /// Expands the selection to the entire soft-wrapped line.
    ///
    /// If multiple lines are already selected, expands the selection to
    /// encompass all soft-wrapped lines that intersected with the prior
    /// selection.  If the selection is a caret is on a soft line break, uses
    /// the affinity of the caret to determine which of the two lines to select.
    /// `SelectLine` should be idempotent: it should never expand onto adjacent
    /// lines.
    SelectLine,

    /// Expands the selection to the entire hard-wrapped line.
    ///
    /// If multiple lines are already selected, expands the selection to
    /// encompass all hard-wrapped lines that intersected with the prior
    /// selection.  `SelectParagraph` should be idempotent: it should never
    /// expand onto adjacent lines.
    SelectParagraph,

    /// Expands the selection to the entire word.
    ///
    /// If multiple words are already selected, expands the selection to
    /// encompass all words that intersected with the prior selection.  If the
    /// selection is a caret is on a word boundary, selects the word downstream
    /// of the caret.  `SelectWord` should be idempotent: it should never expand
    /// onto adjacent words.
    ///
    /// For more information on what these so-called "words" are, see
    /// [Unicode Text Segmentation](https://unicode.org/reports/tr29/#Word_Boundaries).
    SelectWord,

    /// Deletes some text.
    ///
    /// If some text is already selected, `Movement` is ignored, and the
    /// selection is deleted.  If the selection's anchor is the same as the
    /// active, then first apply `MoveSelecting(Movement)` and then delete the
    /// resulting selection.
    Delete(Movement),

    /// Delete backwards, potentially breaking graphemes.
    ///
    /// A special kind of backspace that, instead of deleting the entire
    /// grapheme upstream of the caret, may in some cases and character sets
    /// delete a subset of that grapheme's code points.
    DecomposingBackspace,

    /// Maps the characters in the selection to uppercase.
    ///
    /// For more information on case mapping, see the
    /// [Unicode Case Mapping FAQ](https://unicode.org/faq/casemap_charprop.html#7)
    UppercaseSelection,

    /// Maps the characters in the selection to lowercase.
    ///
    /// For more information on case mapping, see the
    /// [Unicode Case Mapping FAQ](https://unicode.org/faq/casemap_charprop.html#7)
    LowercaseSelection,

    /// Maps the characters in the selection to titlecase.
    ///
    /// When calculating whether a character is at the beginning of a word, you
    /// may have to peek outside the selection to other characters in the document.
    ///
    /// For more information on case mapping, see the
    /// [Unicode Case Mapping FAQ](https://unicode.org/faq/casemap_charprop.html#7)
    TitlecaseSelection,

    /// Inserts a newline character into the document.
    InsertNewLine {
        /// If `true`, then always insert a newline, even if normally you
        /// would run a keyboard shortcut attached to the return key, like
        /// sending a message or activating autocomplete.
        ///
        /// On macOS, this is triggered by pressing option-return.
        ignore_hotkey: bool,
        /// Either `U+000A`, `U+2029`, or `U+2028`. For instance, on macOS, control-enter inserts `U+2028`.
        //FIXME: what about windows?
        newline_type: char,
    },

    /// Inserts a tab character into the document.
    InsertTab {
        /// If `true`, then always insert a tab, even if normally you would run
        /// a keyboard shortcut attached to the return key, like indenting a
        /// line or activating autocomplete.
        ///
        /// On macOS, this is triggered by pressing option-tab.
        ignore_hotkey: bool,
    },

    /// Indicates the reverse of inserting tab; corresponds to shift-tab on most
    /// operating systems.
    InsertBacktab,

    InsertSingleQuoteIgnoringSmartQuotes,
    InsertDoubleQuoteIgnoringSmartQuotes,

    /// Scrolls the text field without modifying the selection.
    Scroll(VerticalMovement),

    /// Centers the selection vertically in the text field.
    ///
    /// The average of the anchor's y and the active's y should be exactly
    /// halfway down the field.  If the selection is taller than the text
    /// field's visible height, then instead scrolls the minimum distance such
    /// that the text field is completely vertically filled by the selection.
    ScrollToSelection,

    /// Sets the writing direction of the selected text or caret.
    SetSelectionWritingDirection(WritingDirection),

    /// Sets the writing direction of all paragraphs that partially or fully
    /// intersect with the selection or caret.
    SetParagraphWritingDirection(WritingDirection),

    /// Cancels the current window or operation.
    ///
    /// Triggered on most operating systems with escape.
    Cancel,
}
