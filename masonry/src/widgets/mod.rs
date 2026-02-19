// Copyright 2018 the Xilem Authors and the Druid Authors
// SPDX-License-Identifier: Apache-2.0

//! Common widgets.

mod align;
mod badge;
mod badged;
mod button;
mod canvas;
mod checkbox;
mod collapse_panel;
mod disclosure_button;
mod divider;
mod flex;
mod grid;
mod image;
mod indexed_stack;
mod label;
mod passthrough;
mod portal;
mod progress_bar;
mod prose;
mod radio_button;
mod radio_group;
mod resize_observer;
mod scroll_bar;
mod sized_box;
mod slider;
mod spinner;
mod split;
mod switch;
mod text_area;
mod text_input;
mod variable_label;
mod virtual_scroll;
mod zstack;

// TODO - Split off widgets and other exports?
// (e.g. actions, param types)

pub use self::align::*;
pub use self::badge::*;
pub use self::badged::*;
pub use self::button::*;
pub use self::canvas::*;
pub use self::checkbox::*;
pub use self::collapse_panel::*;
pub use self::disclosure_button::*;
pub use self::divider::*;
pub use self::flex::*;
pub use self::grid::*;
pub use self::image::*;
pub use self::indexed_stack::*;
pub use self::label::*;
pub use self::passthrough::*;
pub use self::portal::*;
pub use self::progress_bar::*;
pub use self::prose::*;
pub use self::radio_button::*;
pub use self::radio_group::*;
pub use self::resize_observer::*;
pub use self::scroll_bar::*;
pub use self::sized_box::*;
pub use self::slider::*;
pub use self::spinner::*;
pub use self::split::*;
pub use self::switch::*;
pub use self::text_area::*;
pub use self::text_input::*;
pub use self::variable_label::*;
pub use self::virtual_scroll::*;
pub use self::zstack::*;
