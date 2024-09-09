// Copyright 2022 the Xilem Authors
// SPDX-License-Identifier: Apache-2.0

use std::any::Any;

use crate::event::PointerButton;

// TODO - Refactor - See issue https://github.com/linebender/xilem/issues/335

// TODO - TextCursor changed, ImeChanged, EnterKey, MouseEnter
#[non_exhaustive]
#[allow(missing_docs)]
/// Events from UI elements.
///
/// Note: Actions are still a WIP feature.
pub enum Action {
    ButtonPressed(PointerButton),
    TextChanged(String),
    TextEntered(String),
    CheckboxChecked(bool),
    VariableDrag(f64, f64),
    // FIXME - This is a huge hack
    Other(Box<dyn Any + Send>),
}

impl PartialEq for Action {
    fn eq(&self, other: &Self) -> bool {
        match (self, other) {
            (Self::ButtonPressed(l_button), Self::ButtonPressed(r_button)) => l_button == r_button,
            (Self::TextChanged(l0), Self::TextChanged(r0)) => l0 == r0,
            (Self::TextEntered(l0), Self::TextEntered(r0)) => l0 == r0,
            (Self::CheckboxChecked(l0), Self::CheckboxChecked(r0)) => l0 == r0,
            // FIXME
            // (Self::Other(val_l), Self::Other(val_r)) => false,
            _ => false,
        }
    }
}

impl std::fmt::Debug for Action {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            Self::ButtonPressed(button) => f.debug_tuple("ButtonPressed").field(button).finish(),
            Self::TextChanged(text) => f.debug_tuple("TextChanged").field(text).finish(),
            Self::TextEntered(text) => f.debug_tuple("TextEntered").field(text).finish(),
            Self::CheckboxChecked(b) => f.debug_tuple("CheckboxChecked").field(b).finish(),
            Self::VariableDrag(x, y) => f.debug_tuple("VariableDrag").field(x).field(y).finish(),
            Self::Other(_) => write!(f, "Other(...)"),
        }
    }
}
