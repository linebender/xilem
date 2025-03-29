// Copyright 2025 the Xilem Authors
// SPDX-License-Identifier: Apache-2.0

//! Types and logic commonly used across widgets.
//!
//! See [properties documentation](crate::doc::doc_03_implementing_container_widget) for details.

#![allow(
    missing_docs,
    reason = "A lot of properties and especially their fields are self-explanatory."
)]

mod background_color;
mod border_color;
mod border_width;
mod corner_radius;
mod padding;

pub use background_color::BackgroundColor;
pub use border_color::BorderColor;
pub use border_width::BorderWidth;
pub use corner_radius::CornerRadius;
pub use padding::Padding;
