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

use masonry_core::core::{Properties, Widget, WidgetId, WidgetOptions, WidgetPod};

// TODO - Split off into separate file

/// External trait implemented for all widgets.
///
/// Implements helper methods useful for unit testing.
pub trait TestWidgetExt: Widget + Sized + 'static {
    /// Wrap this widget in a [`Recorder`] that records all method calls.
    fn record(self, recording: &Recording) -> Recorder<Self> {
        Recorder::new(self, recording)
    }

    /// Wrap this widget in a [`WrapperWidget`] with the given id.
    fn with_id(self, id: WidgetId) -> WrapperWidget {
        let child = WidgetPod::new_with_id(self, id).erased();
        WrapperWidget::new_pod(child)
    }

    /// Wrap this widget in a [`WrapperWidget`] with the given [`Properties`].
    fn with_props(self, props: Properties) -> WrapperWidget {
        let child = WidgetPod::new_with(
            Box::new(self),
            WidgetId::next(),
            WidgetOptions::default(),
            props,
        )
        .erased();
        WrapperWidget::new_pod(child)
    }
}

impl<W: Widget + 'static> TestWidgetExt for W {}

/// Convenience function to return an array of unique widget ids.
pub fn widget_ids<const N: usize>() -> [WidgetId; N] {
    std::array::from_fn(|_| WidgetId::next())
}
