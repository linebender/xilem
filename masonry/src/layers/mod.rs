// Copyright 2025 the Xilem Authors
// SPDX-License-Identifier: Apache-2.0

//! A list of widgets implementing the [`Layer`](crate::core::Layer) trait.

#![expect(
    missing_debug_implementations,
    reason = "Widgets are not expected to implement Debug"
)]

mod selector_menu;
mod tooltip;

pub use selector_menu::*;
pub use tooltip::*;
