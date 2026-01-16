// Copyright 2025 the Xilem Authors
// SPDX-License-Identifier: Apache-2.0

//! Types and logic commonly used across widgets.
//!
//! See [properties documentation](crate::doc::implementing_container_widget) for details.

mod background;
mod border_color;
mod border_width;
mod box_shadow;
mod checkmark;
mod content_color;
mod corner_radius;
mod gap;
mod line_breaking;
mod object_fit;
mod padding;
mod placeholder_color;
mod progress_bar;
mod selection;
mod slider;

pub mod types;

pub use background::*;
pub use border_color::*;
pub use border_width::*;
pub use box_shadow::*;
pub use checkmark::*;
pub use content_color::*;
pub use corner_radius::*;
pub use gap::*;
pub use line_breaking::*;
pub use object_fit::*;
pub use padding::*;
pub use placeholder_color::*;
pub use progress_bar::*;
pub use selection::*;
pub use slider::*;

pub use masonry_core::properties::*;
