// Copyright 2022 the Xilem Authors
// SPDX-License-Identifier: Apache-2.0

use anymore::AnyDebug;

use crate::core::PointerButton;

// TODO - Replace actions with an associated type on the Widget trait
// See https://github.com/linebender/xilem/issues/664

// TODO - TextCursor changed, ImeChanged, EnterKey, MouseEnter
/// Events from UI elements.
///
/// Note: Actions are still a WIP feature.
#[non_exhaustive]
#[derive(Debug)]
pub enum Action {
    /// A button was pressed.
    ///
    /// Some presses are triggered without a pointer button;
    /// for example, a touch screen does not exercise buttons.
    /// In these cases, `None` will be the value here.
    ButtonPressed(Option<PointerButton>),
    /// Text changed.
    TextChanged(String),
    /// Text entered.
    TextEntered(String),
    /// A checkbox was toggled.
    CheckboxToggled(bool),
    // FIXME - This is a huge hack
    /// Other.
    Other(Box<dyn AnyDebug + Send>),
}

impl PartialEq for Action {
    fn eq(&self, other: &Self) -> bool {
        match (self, other) {
            (Self::ButtonPressed(l_button), Self::ButtonPressed(r_button)) => l_button == r_button,
            (Self::TextChanged(l0), Self::TextChanged(r0)) => l0 == r0,
            (Self::TextEntered(l0), Self::TextEntered(r0)) => l0 == r0,
            (Self::CheckboxToggled(l0), Self::CheckboxToggled(r0)) => l0 == r0,
            // FIXME
            // (Self::Other(val_l), Self::Other(val_r)) => false,
            _ => false,
        }
    }
}
