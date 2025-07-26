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
mod corner_radius;
mod padding;
mod progress_bar;
mod text_color;

pub mod types;

pub use background::{ActiveBackground, Background, DisabledBackground};
pub use border_color::{BorderColor, HoveredBorderColor};
pub use border_width::BorderWidth;
pub use box_shadow::BoxShadow;
pub use checkmark::{CheckmarkColor, CheckmarkStrokeWidth, DisabledCheckmarkColor};
pub use corner_radius::CornerRadius;
pub use padding::Padding;
pub use progress_bar::BarColor;
pub use text_color::{DisabledTextColor, TextColor};
