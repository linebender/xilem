use std::collections::VecDeque;

use crate::{WidgetId, WindowId};

// TODO - Rename
// TODO - Figure out what these actions should be

// TODO - TextCursor changed, ImeChanged, EnterKey, MouseEnter
#[derive(Debug, PartialEq, Eq)]
pub enum Action {
    ButtonPressed,
    TextChanged(String),
}

/// Our queue type
pub(crate) type ActionQueue = VecDeque<(Action, WidgetId, WindowId)>;
