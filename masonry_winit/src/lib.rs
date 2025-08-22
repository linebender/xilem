// Copyright 2025 the Xilem Authors
// SPDX-License-Identifier: Apache-2.0

//! This is the [Winit][winit] backend for the [Masonry] GUI framework.
//!
//! See [Masonry's documentation] for more details, examples and resources.
//!
//! # Example
//!
//! ```rust
//! use masonry::core::{ErasedAction, NewWidget, Widget, WidgetId, WidgetPod};
//! use masonry::dpi::LogicalSize;
//! use masonry::theme::default_property_set;
//! use masonry_winit::app::{AppDriver, DriverCtx, NewWindow, WindowId};
//! use masonry_winit::winit::window::Window;
//!
//! struct Driver {
//!     // ...
//! }
//!
//! impl AppDriver for Driver {
//!     fn on_action(
//!         &mut self,
//!         window_id: WindowId,
//!         ctx: &mut DriverCtx<'_, '_>,
//!         widget_id: WidgetId,
//!         action: ErasedAction,
//!     ) {
//!         // ...
//!     }
//! }
//!
//! fn main() {
//!     let main_widget = {
//!         // ...
//!         # masonry::widgets::Label::new("hello")
//!     };
//!
//!     let window_size = LogicalSize::new(400.0, 400.0);
//!     let window_attributes = masonry_winit::winit::window::WindowAttributes::default()
//!         .with_title("My Masonry App")
//!         .with_resizable(true)
//!         .with_min_inner_size(window_size);
//!
//!     let driver = Driver {
//!         // ...
//!     };
//!     # return;
//!     let event_loop = masonry_winit::app::EventLoop::builder()
//!         .build()
//!         .unwrap();
//!     masonry_winit::app::run_with(
//!         event_loop,
//!         vec![NewWindow::new(
//!             window_attributes,
//!             NewWidget::new(main_widget).erased(),
//!         )],
//!         driver,
//!         default_property_set(),
//!     )
//!     .unwrap();
//! }
//! ```
//!
//! (See the Masonry documentation for more detailed examples.)
//!
//! [Masonry's documentation]: https://docs.rs/masonry
//! [Masonry]: https://crates.io/crates/masonry

// LINEBENDER LINT SET - lib.rs - v3
// See https://linebender.org/wiki/canonical-lints/
// These lints shouldn't apply to examples or tests.
#![cfg_attr(not(test), warn(unused_crate_dependencies))]
// These lints shouldn't apply to examples.
#![warn(clippy::print_stdout, clippy::print_stderr)]
// Targeting e.g. 32-bit means structs containing usize can give false positives for 64-bit.
#![cfg_attr(target_pointer_width = "64", warn(clippy::trivially_copy_pass_by_ref))]
// END LINEBENDER LINT SET
#![cfg_attr(docsrs, feature(doc_auto_cfg))]
#![expect(
    clippy::needless_doctest_main,
    reason = "Having a main function is a deliberate part of the root doc."
)]
#![expect(missing_debug_implementations, reason = "Deferred: Noisy")]

// TODO - Add logo

mod app_driver;
mod convert_winit_event;
mod event_loop_runner;

pub use winit;

/// Types needed for running a Masonry app.
pub mod app {
    pub use super::app_driver::{AppDriver, DriverCtx, WindowId};
    pub use super::event_loop_runner::{
        EventLoop, EventLoopBuilder, EventLoopProxy, MasonryState, MasonryUserEvent, NewWindow,
        run, run_with,
    };

    pub(crate) use super::convert_winit_event::{
        masonry_resize_direction_to_winit, winit_ime_to_masonry,
    };
}
