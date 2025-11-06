// Copyright 2020 the Xilem Authors and the Druid Authors
// SPDX-License-Identifier: Apache-2.0

// After you edit the crate's doc comment, run this command, then check README.md for any missing links
// cargo rdme --workspace-project=masonry_testing

//! Headless runner for testing [Masonry](https://docs.rs/masonry/latest/) applications.
//!
//! The primary type from this crate is [`TestHarness`], which creates a host for any [Widget].
//! The widget can of course have children, which allows this crate to be used for testing entire applications.
//!
//! The testing harness can:
//!
//! - Simulate any external event which Masonry handles, including mouse movement, key presses, text input, accessibility events.
//! - Control the flow of time to the application (i.e. for testing animations).
//! - Take screenshots of the application, save these to a file, and ensure that these are up-to-date.
//!   See [Screenshots](#screenshots) for more details.
//!
//! <!-- Masonry itself depends on Masonry Testing, so we can't use an intra-doc link here. -->
//! Testing in Masonry is also documented in the [Testing widgets in Masonry](https://docs.rs/masonry/latest/masonry/doc/doc_04_testing_widget/index.html)
//! chapter in Masonry's book.
//!
//! This crate can be accessed for applications using Masonry as `masonry::testing`, if Masonry's `testing` feature is enabled.
//! For applications which are using only [Masonry Core](masonry_core), you should depend on `masonry_testing` directly.
//!
//! # Screenshots
//!
//! Tests using `TestHarness` can include snapshot steps by using the [`assert_render_snapshot`] screenshot.
//! This renders the application being tested, then compares it against the png file with the given name
//! from the `screenshots` folder (in the package being tested, i.e. adjacent to its `Cargo.toml` file).
//!
//! Masonry Testing will update the reference file when the `MASONRY_TEST_BLESS` environment variable has a value of `1`.
//! This can be used if the file doesn't exist, or there's an expected difference.
//! The screenshots are losslessly compressed (using [oxipng]) and limited to a small maximum file size (this
//! limit has an escape hatch).
//! This ensures that the screenshots are small enough to embed in a git repository with limited risk
//! of clone times growing unreasonably.
//! UI screenshots compress well, so we expect this to be scalable.
//!
//! For repositories hosted on GitHub, this scheme also allows for including screenshots of your app or
//! widgets in hosted documentation, although we haven't documented this publicly yet.
//!
//! # Examples
//!
//! For examples of this crate in use
//!
//! - To test applications: see the tests in Masonry's examples.
//! - To test widgets: see the `tests` module in each widget in Masonry.

// TODO: Remove any items listed as "Deferred"
#![expect(missing_debug_implementations, reason = "Deferred: Noisy")]

mod assert_any;
mod assert_debug_panics;
mod debug_name;
mod harness;
mod modular_widget;
mod recorder_widget;
mod screenshots;
mod wrapper_widget;

pub use assert_any::{assert_all, assert_any, assert_none};
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
