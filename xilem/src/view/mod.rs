// Copyright 2024 the Xilem Authors
// SPDX-License-Identifier: Apache-2.0

//! Views for the widgets which are built-in to Masonry. These are the primitives your Xilem app's view tree will generally be constructed from.

// TODO - Remove this re-export, and change code importing it
// to import `masonry::core::Axis` directly.
// See https://github.com/linebender/xilem/issues/1254
pub use masonry::core::Axis;

mod task;
pub use task::*;

mod worker;
pub use worker::*;

mod button;
pub use button::*;

mod checkbox;
pub use checkbox::*;

mod flex;
pub use flex::*;

mod grid;
pub use grid::*;

mod sized_box;
pub use sized_box::*;

mod slider;
pub use slider::*;

mod spinner;
pub use spinner::*;

mod image;
pub use image::*;

mod indexed_stack;
pub use indexed_stack::*;

mod label;
pub use label::*;

mod variable_label;
pub use variable_label::*;

mod progress_bar;
pub use progress_bar::*;

mod prop;
pub use prop::*;

mod prose;
pub use prose::*;

mod text_input;
pub use text_input::*;

mod virtual_scroll;
pub use virtual_scroll::*;

mod portal;
pub use portal::*;

mod zstack;
pub use zstack::*;

mod transform;
pub use transform::*;

mod split;
pub use split::*;
