// Copyright 2024 the Xilem Authors
// SPDX-License-Identifier: Apache-2.0

//! Views for the widgets which are built-in to Masonry. These are the primitives your Xilem app's view tree will generally be constructed from.

// TODO - Remove this re-export, and change code importing it
// to import `masonry::core::Axis` directly.
// See https://github.com/linebender/xilem/issues/1254
pub use masonry::core::Axis;

mod button;
mod canvas;
mod checkbox;
mod flex;
mod grid;
mod image;
mod indexed_stack;
mod label;
mod portal;
mod progress_bar;
mod prop;
mod prose;
mod sized_box;
mod slider;
mod spinner;
mod split;
mod task;
mod text_input;
mod transform;
mod variable_label;
mod virtual_scroll;
mod worker;
mod zstack;

pub use self::button::*;
pub use self::canvas::*;
pub use self::checkbox::*;
pub use self::flex::*;
pub use self::grid::*;
pub use self::image::*;
pub use self::indexed_stack::*;
pub use self::label::*;
pub use self::portal::*;
pub use self::progress_bar::*;
pub use self::prop::*;
pub use self::prose::*;
pub use self::sized_box::*;
pub use self::slider::*;
pub use self::spinner::*;
pub use self::split::*;
pub use self::task::*;
pub use self::text_input::*;
pub use self::transform::*;
pub use self::variable_label::*;
pub use self::virtual_scroll::*;
pub use self::worker::*;
pub use self::zstack::*;
