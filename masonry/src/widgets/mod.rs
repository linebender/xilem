// Copyright 2018 the Xilem Authors and the Druid Authors
// SPDX-License-Identifier: Apache-2.0

//! Common widgets.

mod align;
mod button;
mod checkbox;
mod flex;
mod grid;
mod image;
mod indexed_stack;
mod label;
mod portal;
mod progress_bar;
mod prose;
mod scroll_bar;
mod slider;
mod sized_box;
mod spinner;
mod split;
mod text_area;
mod text_input;
mod variable_label;
mod virtual_scroll;
mod zstack;

pub use self::align::Align;
pub use self::button::{Button, ButtonPress};
pub use self::checkbox::{Checkbox, CheckboxToggled};
pub use self::flex::{Flex, FlexParams};
pub use self::grid::{Grid, GridParams};
pub use self::image::Image;
pub use self::indexed_stack::IndexedStack;
pub use self::label::Label;
pub use self::portal::Portal;
pub use self::progress_bar::ProgressBar;
pub use self::prose::Prose;
pub use self::scroll_bar::ScrollBar;
pub use self::slider::Slider;
pub use self::sized_box::SizedBox;
pub use self::spinner::Spinner;
pub use self::split::{Split, ceil_length};
pub use self::text_area::{InsertNewline, TextAction, TextArea};
pub use self::text_input::TextInput;
pub use self::variable_label::VariableLabel;
pub use self::virtual_scroll::{VirtualScroll, VirtualScrollAction};
pub use self::zstack::{ChildAlignment, ZStack};
