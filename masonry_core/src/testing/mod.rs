// Copyright 2020 the Xilem Authors and the Druid Authors
// SPDX-License-Identifier: Apache-2.0

//! Helper tools for writing unit tests.

mod harness;
mod helper_widgets;
mod screenshots;
mod snapshot_utils;

pub use harness::{TestHarness, TestHarnessParams};
pub use helper_widgets::{ModularWidget, Record, Recorder, Recording, ReplaceChild, TestWidgetExt};

use crate::core::WidgetId;

/// Convenience function to return an array of unique widget ids.
pub fn widget_ids<const N: usize>() -> [WidgetId; N] {
    std::array::from_fn(|_| WidgetId::next())
}
