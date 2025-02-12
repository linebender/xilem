// Copyright 2018 the Xilem Authors and the Druid Authors
// SPDX-License-Identifier: Apache-2.0

//! Common widgets.

// We use allow because expect(missing_docs) is noisy with rust-analyzer.
#![allow(missing_docs, reason = "We have many as-yet undocumented items")]

#[cfg(test)]
mod tests;

mod align;
mod button;
mod button9;
mod checkbox;
mod flex;
mod grid;
mod image;
mod label;
mod portal;
mod progress_bar;
mod prose;
mod root_widget;
mod scroll_bar;
mod sized_box;
mod spinner;
mod split;
mod text_area;
mod textbox;
mod variable_label;
mod zstack;

pub use self::align::Align;
pub use self::button::Button;
pub use self::button9::{Button9, Label9, LabelOpt, Pad9};
pub use self::checkbox::Checkbox;
pub use self::flex::{Axis, CrossAxisAlignment, Flex, FlexParams, MainAxisAlignment};
pub use self::grid::{Grid, GridParams};
pub use self::image::Image;
pub use self::label::{Label, LineBreaking};
pub use self::portal::Portal;
pub use self::progress_bar::ProgressBar;
pub use self::prose::Prose;
pub use self::root_widget::RootWidget;
pub use self::scroll_bar::ScrollBar;
pub use self::sized_box::{Padding, SizedBox};
pub use self::spinner::Spinner;
pub use self::split::Split;
pub use self::text_area::TextArea;
pub use self::textbox::Textbox;
pub use self::variable_label::VariableLabel;
pub use self::zstack::{Alignment, ChildAlignment, HorizontalAlignment, VerticalAlignment, ZStack};
