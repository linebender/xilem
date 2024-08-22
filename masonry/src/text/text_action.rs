use parley::layout::Affinity;
use winit::event::KeyEvent;
use winit::keyboard::{KeyCode, ModifiersState, PhysicalKey};

use crate::text::movement::Movement;

#[derive(Debug, Clone, PartialEq, Eq, PartialOrd, Ord, Hash)]
pub enum TextAction {
    // Arrows, Home, End, etc
    Move(Movement),
    // Ctrl+A
    SelectAll,
    // Shift+Arrows, Shift+Home, Shift+End, etc
    Select(Movement),
    // Backspace, Delete, Ctrl+Backspace, Ctrl+Delete
    SelectAndDelete(Movement),
    // Regular input, paste, IME
    Splice(String),
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

pub fn convert(event: KeyEvent, mods: ModifiersState) -> Option<TextAction> {
    #[allow(unused)]
    let (shift, ctrl, cmd) = (mods.shift_key(), mods.control_key(), mods.super_key());
    let action_mod = if cfg!(target_os = "macos") { cmd } else { ctrl };

    let PhysicalKey::Code(code) = event.physical_key else {
        return None;
    };
    #[rustfmt::skip]
    let action = match code {
        // TODO
        // KeyCode::KeyC
        // KeyCode::KeyX
        // KeyCode::KeyV
        KeyCode::KeyA if !shift && action_mod => {
            TextAction::SelectAll
        }
        KeyCode::ArrowLeft if action_mod => {
            TextAction::move_or_select(Movement::Word(Affinity::Upstream), shift)
        }
        KeyCode::ArrowRight if action_mod => {
            TextAction::move_or_select(Movement::Word(Affinity::Downstream), shift)
        }
        KeyCode::Home if action_mod => {
            TextAction::move_or_select(Movement::DocumentStart, shift)
        }
        KeyCode::End if action_mod => {
            TextAction::move_or_select(Movement::DocumentEnd, shift)
        }
        KeyCode::Delete if action_mod && shift => {
            TextAction::SelectAndDelete(Movement::Line(Affinity::Downstream))
        }
        KeyCode::Backspace if action_mod && shift => {
            TextAction::SelectAndDelete(Movement::Line(Affinity::Upstream))
        }
        KeyCode::Delete if action_mod => {
            TextAction::SelectAndDelete(Movement::Word(Affinity::Downstream))
        }
        KeyCode::Backspace if action_mod => {
            TextAction::SelectAndDelete(Movement::Word(Affinity::Upstream))
        }
        KeyCode::ArrowLeft => {
            TextAction::move_or_select(Movement::Grapheme(Affinity::Upstream), shift)
        }
        KeyCode::ArrowRight => {
            TextAction::move_or_select(Movement::Grapheme(Affinity::Downstream), shift)
        }
        KeyCode::ArrowUp => {
            TextAction::move_or_select(Movement::LineUp(1), shift)
        }
        KeyCode::ArrowDown => {
            TextAction::move_or_select(Movement::LineDown(1), shift)
        }
        KeyCode::Home => {
            TextAction::move_or_select(Movement::Line(Affinity::Upstream), shift)
        }
        KeyCode::End => {
            TextAction::move_or_select(Movement::Grapheme(Affinity::Downstream), shift)
        }
        KeyCode::Delete => {
            TextAction::SelectAndDelete(Movement::Grapheme(Affinity::Downstream))
        }
        KeyCode::Backspace => {
            TextAction::SelectAndDelete(Movement::Grapheme(Affinity::Upstream))
        }
        _ => {
            let text = event.text.map(|text| text.to_string()).unwrap_or_default();
            TextAction::Splice(text)
        }
    };
    Some(action)
}

/*
 */
