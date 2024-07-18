// Copyright 2020 the Xilem Authors and the Druid Authors
// SPDX-License-Identifier: Apache-2.0

//! Helper tools for writing unit tests.

#![cfg(not(tarpaulin_include))]

#[cfg(not(tarpaulin_include))]
mod harness;
#[cfg(not(tarpaulin_include))]
mod helper_widgets;
#[cfg(not(tarpaulin_include))]
mod screenshots;
#[cfg(not(tarpaulin_include))]
mod snapshot_utils;

pub use harness::{TestHarness, HARNESS_DEFAULT_SIZE};
pub use helper_widgets::{ModularWidget, Record, Recorder, Recording, ReplaceChild, TestWidgetExt};

use crate::WidgetId;

/// Convenience function to return an arrays of unique widget ids.
pub fn widget_ids<const N: usize>() -> [WidgetId; N] {
    std::array::from_fn(|_| WidgetId::next())
}
