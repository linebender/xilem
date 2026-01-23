// Copyright 2025 the Xilem Authors
// SPDX-License-Identifier: Apache-2.0

//! Types and logic commonly used across widgets.
//!
//! See [properties documentation](crate::doc::implementing_container_widget) for details.

mod checkmark;
mod content_color;
mod gap;
mod line_breaking;
mod object_fit;
mod placeholder_color;
mod progress_bar;
mod selection;
mod slider;
mod switch;

pub mod types;

pub use checkmark::*;
pub use content_color::*;
pub use gap::*;
pub use line_breaking::*;
pub use object_fit::*;
pub use placeholder_color::*;
pub use progress_bar::*;
pub use selection::*;
pub use slider::*;
pub use switch::*;

pub use masonry_core::properties::*;
