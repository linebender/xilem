// Copyright 2020 the Xilem Authors and the Druid Authors
// SPDX-License-Identifier: Apache-2.0

//! Helper tools for writing unit tests for Masonry.

// TODO: Remove any items listed as "Deferred"
#![expect(missing_debug_implementations, reason = "Deferred: Noisy")]

mod harness;
mod modular_widget;
mod recorder_widget;
mod screenshots;
mod wrapper_widget;

pub use harness::{PRIMARY_MOUSE, TestHarness, TestHarnessParams};
pub use modular_widget::ModularWidget;
pub use recorder_widget::{Record, Recorder, Recording};
pub use wrapper_widget::WrapperWidget;

use masonry_core::core::{Widget, WidgetId};

/// External trait implemented for all widgets.
///
/// Implements helper methods useful for unit testing.
pub trait TestWidgetExt: Widget + Sized + 'static {
    // TODO - Remove, keep only record()
    /// Wrap this widget in a [`Recorder`] that records all method calls.
    ///
    /// Takes a reference to a [`Recording`] to store records in.
    fn record_with(self, recording: &Recording) -> Recorder<Self> {
        Recorder::new(self, recording)
    }

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
