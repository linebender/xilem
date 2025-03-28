// Copyright 2025 the Xilem Authors
// SPDX-License-Identifier: Apache-2.0

//! Masonry gives you a platform to create windows (using [winit] as a backend) each with a tree of widgets. It also gives you tools to inspect that widget tree at runtime, write unit tests on it, and generally have an easier time debugging and maintaining your app.
//!
//! The framework is not opinionated about what your user-facing abstraction will be: you can implement immediate-mode GUI, the Elm architecture, functional reactive GUI, etc, on top of Masonry.
//!
//! See [Xilem] as an example of reactive UI built on top of Masonry.
//!
//! Masonry was originally a fork of [Druid] that emerged from discussions within the Linebender community about what it would look like to turn Druid into a foundational library.
//!
//! Masonry can currently be considered to be in an alpha state.
//! Lots of things need improvements, e.g. text input is janky and snapshot testing is not consistent across platforms.
//!
//! ## Example
//!
//! The to-do-list example looks like this:
//!
//! ```
//! use masonry::app::{AppDriver, DriverCtx};
//! use masonry::core::{Action, Widget, WidgetId};
//! use masonry::dpi::LogicalSize;
//! use masonry::widgets::{Button, Flex, Label, Portal, RootWidget, Textbox};
//! use winit::window::Window;
//!
//! struct Driver {
//!     next_task: String,
//! }
//!
//! impl AppDriver for Driver {
//!     fn on_action(&mut self, ctx: &mut DriverCtx<'_>, _widget_id: WidgetId, action: Action) {
//!         match action {
//!             Action::ButtonPressed(_) => {
//!                 ctx.render_root().edit_root_widget(|mut root| {
//!                     let mut root = root.downcast::<RootWidget<Portal<Flex>>>();
//!                     let mut portal = RootWidget::child_mut(&mut root);
//!                     let mut flex = Portal::child_mut(&mut portal);
//!                     Flex::add_child(&mut flex, Label::new(self.next_task.clone()));
//!                 });
//!             }
//!             Action::TextChanged(new_text) => {
//!                 self.next_task = new_text.clone();
//!             }
//!             _ => {}
//!         }
//!     }
//! }
//!
//! fn main() {
//!     const VERTICAL_WIDGET_SPACING: f64 = 20.0;
//!
//!     let main_widget = Portal::new(
//!         Flex::column()
//!             .with_child(
//!                 Flex::row()
//!                     .with_flex_child(Textbox::new(""), 1.0)
//!                     .with_child(Button::new("Add task")),
//!             )
//!             .with_spacer(VERTICAL_WIDGET_SPACING),
//!     );
//!
//!     let window_size = LogicalSize::new(400.0, 400.0);
//!     let window_attributes = Window::default_attributes()
//!         .with_title("To-do list")
//!         .with_resizable(true)
//!         .with_min_inner_size(window_size);
//!
//!     # return;
//!     masonry::app::run(
//!         masonry::app::EventLoop::with_user_event(),
//!         window_attributes,
//!         RootWidget::new(main_widget),
//!         Driver {
//!             next_task: String::new(),
//!         },
//!     )
//!     .unwrap();
//! }
//! ```
//!
//! For more information, see [the documentation module](masonry_core::doc).
//!
//! ### Crate feature flags
//!
//! The following feature flags are available:
//!
//! - `tracy`: Enables creating output for the [Tracy](https://github.com/wolfpld/tracy) profiler using [`tracing-tracy`][tracing_tracy].
//!   This can be used by installing Tracy and connecting to a Masonry with this feature enabled.
//!
//! ### Debugging features
//!
//! Masonry apps currently ship with two debugging features built in:
//! - A rudimentary widget inspector - toggled by F11 key.
//! - A debug mode painting widget layout rectangles - toggled by F12 key.
//!
//! [winit]: https://crates.io/crates/winit
//! [Druid]: https://crates.io/crates/druid
//! [Xilem]: https://crates.io/crates/xilem
//! [tracing_tracy]: https://crates.io/crates/tracing-tracy
// TODO: Add screenshot. This can't use include_screenshot as that doesn't work with cargo-rdme
// See https://github.com/linebender/xilem/issues/851

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
#![cfg_attr(
    test,
    expect(
        unused_crate_dependencies,
        reason = "False-positive with dev-dependencies only used in examples"
    )
)]
#![expect(clippy::needless_doctest_main, reason = "Deferred: Noisy")]
#![expect(missing_debug_implementations, reason = "Deferred: Noisy")]
#![expect(
    clippy::shadow_unrelated,
    reason = "Potentially controversial code style"
)]

// TODO - Add logo

pub use masonry_core::*;

// TODO - Restructure re-exports.

mod app_driver;
mod convert_winit_event;
mod event_loop_runner;

/// Types needed for running a Masonry app.
pub mod app {
    pub use masonry_core::app::*;

    pub use super::app_driver::{AppDriver, DriverCtx};
    pub use super::event_loop_runner::{
        EventLoop, EventLoopBuilder, EventLoopProxy, MasonryState, MasonryUserEvent, run, run_with,
    };

    pub(crate) use super::convert_winit_event::{
        masonry_resize_direction_to_winit, winit_force_to_masonry, winit_ime_to_masonry,
        winit_key_event_to_kbt, winit_modifiers_to_kbt_modifiers, winit_mouse_button_to_masonry,
    };
}
