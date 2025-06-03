// Copyright 2020 the Xilem Authors and the Druid Authors
// SPDX-License-Identifier: Apache-2.0

//! Helper tools for writing unit tests.

#![expect(clippy::allow_attributes_without_reason, reason = "Deferred: Noisy")]
// TODO: Remove any items listed as "Deferred"
#![cfg_attr(not(debug_assertions), expect(unused, reason = "Deferred: Noisy"))]
#![expect(missing_debug_implementations, reason = "Deferred: Noisy")]
#![expect(clippy::cast_possible_truncation, reason = "Deferred: Noisy")]
#![expect(clippy::missing_assert_message, reason = "Deferred: Noisy")]
#![expect(elided_lifetimes_in_paths, reason = "Deferred: Noisy")]
#![expect(unreachable_pub, reason = "Potentially controversial code style")]
#![expect(clippy::todo, reason = "We have a lot of 'real' todos")]

mod harness;
mod modular_widget;
mod recorder_widget;
mod screenshots;
mod wrapper_widget;

pub use harness::{PRIMARY_MOUSE, TestHarness, TestHarnessParams};
pub use modular_widget::ModularWidget;
pub use recorder_widget::{Record, Recorder, Recording};
pub use wrapper_widget::WrapperWidget;

use masonry::core::{Widget, WidgetId, WidgetPod};

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
}

impl<W: Widget + 'static> TestWidgetExt for W {}

/// Convenience function to return an array of unique widget ids.
pub fn widget_ids<const N: usize>() -> [WidgetId; N] {
    std::array::from_fn(|_| WidgetId::next())
}
