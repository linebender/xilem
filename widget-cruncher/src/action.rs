use std::collections::VecDeque;

use crate::{WidgetId, WindowId};

// TODO - Rename
// TODO - Figure out what these actions should be

// TODO - TextCursor changed, ImeChanged, EnterKey, MouseEnter
#[derive(Debug, PartialEq, Eq)]
#[non_exhaustive]
pub enum Action {
    ButtonPressed,
    TextChanged(String),
    CheckboxChecked(bool),
}

/// Our queue type
pub(crate) type ActionQueue = VecDeque<(Action, WidgetId, WindowId)>;
