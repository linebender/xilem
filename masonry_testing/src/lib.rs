// Copyright 2020 the Xilem Authors and the Druid Authors
// SPDX-License-Identifier: Apache-2.0

//! Helper tools for writing unit tests for Masonry.

// TODO: Remove any items listed as "Deferred"
#![expect(missing_debug_implementations, reason = "Deferred: Noisy")]

mod assert_debug_panics;
mod debug_name;
mod harness;
mod modular_widget;
mod recorder_widget;
mod screenshots;
mod wrapper_widget;

pub use assert_debug_panics::assert_debug_panics_inner;
pub use debug_name::DebugName;
pub use harness::{PRIMARY_MOUSE, TestHarness, TestHarnessParams};
pub use modular_widget::ModularWidget;
pub use recorder_widget::{Record, Recorder, Recording};
pub use wrapper_widget::WrapperWidget;

use masonry_core::core::{Widget, WidgetId};

/// External trait implemented for all widgets.
///
/// Implements helper methods useful for unit testing.
pub trait TestWidgetExt: Widget + Sized + 'static {
    /// Wrap this widget in a [`Recorder`] that records all method calls.
    fn record(self) -> Recorder<Self> {
        let recording = Recording::default();
        Recorder::new(self, &recording)
    }
}

impl<W: Widget + 'static> TestWidgetExt for W {}

// TODO - We eventually want to remove the ability to reserve widget ids.
// See https://github.com/linebender/xilem/issues/1255
/// Convenience function to return an array of unique widget ids.
pub fn widget_ids<const N: usize>() -> [WidgetId; N] {
    std::array::from_fn(|_| WidgetId::next())
}
