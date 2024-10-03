// Copyright 2024 the Xilem Authors and the Druid Authors
// SPDX-License-Identifier: Apache-2.0

#![expect(unused)]

use parley::layout::cursor::{Selection, VisualMode};
use parley::layout::Affinity;
use parley::style::Brush;
use parley::Layout;
use winit::event::KeyEvent;
use winit::keyboard::{KeyCode, ModifiersState, PhysicalKey};

// TODO - Add ParagraphStart and ParagraphEnd.

/// Different ways a cursor can move.
///
/// This enum tries to match the API of parley::layout::cursor::Selection,
/// and may change in the future.
#[derive(Debug, Clone, PartialEq, Eq, PartialOrd, Ord, Hash)]
pub enum Movement {
    /// Backspace may behave differently than LeftArrow for emoji.
    Backspace,
    GraphemeLeft,
    GraphemeRight,
    WordPrev,
    WordNext,
    LineStart,
    LineEnd,
    LineUp(usize),
    LineDown(usize),
    DocumentStart,
    DocumentEnd,
}

impl Movement {
    pub fn apply_to<B: Brush>(
        &self,
        selection: &Selection,
        layout: &Layout<B>,
        extend: bool,
    ) -> Selection {
        match self {
            Movement::Backspace => {
                // TODO - Use backspace-specific algo.
                selection.previous_visual(layout, VisualMode::Strong, extend)
            }
            Movement::GraphemeLeft => selection.previous_visual(layout, VisualMode::Strong, extend),
            Movement::GraphemeRight => selection.next_visual(layout, VisualMode::Strong, extend),
            Movement::WordPrev => selection.previous_word(layout, extend),
            Movement::WordNext => selection.next_word(layout, extend),
            Movement::LineStart => selection.line_start(layout, extend),
            Movement::LineEnd => selection.line_end(layout, extend),
            Movement::LineUp(count) => selection.move_lines(layout, -(*count as isize), extend),
            Movement::LineDown(count) => selection.move_lines(layout, (*count as isize), extend),
            // TODO - Express this in a less hacky way.
            Movement::DocumentStart => selection.move_lines(&layout, isize::MIN, extend),
            Movement::DocumentEnd => selection.move_lines(&layout, isize::MAX, extend),
        }
    }
}

#[derive(Debug, Clone, PartialEq, Eq, PartialOrd, Ord, Hash)]
pub enum TextAction {
    /// Arrows, Home, End, etc
    Move(Movement),
    /// Ctrl+A
    SelectAll,
    /// Shift+Arrows, Shift+Home, Shift+End, etc
    Select(Movement),
    /// Backspace, Delete, Ctrl+Backspace, Ctrl+Delete.
    ///
    /// If the selection is empty, will extend the cursor with the given movement, the delete the section.
    /// Otherwise, will delete the selection.
    SelectAndDelete(Movement),
    /// Regular input, IME commit
    Splice(String),
    /// IME preedit
    Preedit(String),
    /// Clipboard copy
    Copy,
    /// Clipboard cut
    Cut,
    /// Clipboard paste
    Paste,
}

impl TextAction {
    pub fn move_or_select(movement: Movement, select: bool) -> Self {
        if select {
            TextAction::Select(movement)
        } else {
            TextAction::Move(movement)
        }
    }
}

pub fn convert_key(event: KeyEvent, mods: ModifiersState) -> Option<TextAction> {
    #[allow(unused)]
    let (shift, ctrl, cmd) = (mods.shift_key(), mods.control_key(), mods.super_key());
    let action_mod = if cfg!(target_os = "macos") { cmd } else { ctrl };

    let PhysicalKey::Code(code) = event.physical_key else {
        return None;
    };

    #[rustfmt::skip]
    let action = match code {
        KeyCode::KeyA if !shift && action_mod => {
            TextAction::SelectAll
        }
        KeyCode::ArrowLeft if action_mod => {
            TextAction::move_or_select(Movement::WordPrev, shift)
        }
        KeyCode::ArrowRight if action_mod => {
            TextAction::move_or_select(Movement::WordNext, shift)
        }
        KeyCode::Home if action_mod => {
            TextAction::move_or_select(Movement::DocumentStart, shift)
        }
        KeyCode::End if action_mod => {
            TextAction::move_or_select(Movement::DocumentEnd, shift)
        }
        KeyCode::Delete if action_mod && shift => {
            TextAction::SelectAndDelete(Movement::LineEnd)
        }
        KeyCode::Backspace if action_mod && shift => {
            TextAction::SelectAndDelete(Movement::LineStart)
        }
        KeyCode::Delete if action_mod => {
            TextAction::SelectAndDelete(Movement::WordNext)
        }
        KeyCode::Backspace if action_mod => {
            TextAction::SelectAndDelete(Movement::WordPrev)
        }
        KeyCode::ArrowLeft => {
            TextAction::move_or_select(Movement::GraphemeLeft, shift)
        }
        KeyCode::ArrowRight => {
            TextAction::move_or_select(Movement::GraphemeRight, shift)
        }
        KeyCode::ArrowUp => {
            TextAction::move_or_select(Movement::LineUp(1), shift)
        }
        KeyCode::ArrowDown => {
            TextAction::move_or_select(Movement::LineDown(1), shift)
        }
        KeyCode::Home => {
            TextAction::move_or_select(Movement::LineStart, shift)
        }
        KeyCode::End => {
            TextAction::move_or_select(Movement::LineEnd, shift)
        }
        KeyCode::Delete => {
            TextAction::SelectAndDelete(Movement::GraphemeRight)
        }
        KeyCode::Backspace => {
            TextAction::SelectAndDelete(Movement::Backspace)
        }
        KeyCode::KeyC if action_mod => {
            TextAction::Copy
        }
        KeyCode::KeyX if action_mod => {
            TextAction::Cut
        }
        KeyCode::KeyV if action_mod => {
            TextAction::Paste
        }
        _ => {
            let text = event.text.map(|text| text.to_string()).unwrap_or_default();
            TextAction::Splice(text)
        }
    };
    Some(action)
}

pub fn convert_key_readonly(event: KeyEvent, mods: ModifiersState) -> Option<TextAction> {
    let action = convert_key(event, mods)?;

    match action {
        TextAction::SelectAndDelete(_) => None,
        TextAction::Splice(_) => None,
        TextAction::Cut => None,
        TextAction::Paste => None,
        _ => Some(action),
    }
}
