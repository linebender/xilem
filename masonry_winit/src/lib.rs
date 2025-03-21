// Copyright 2018 the Xilem Authors and the Druid Authors
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
//! For more information, see [the documentation module](crate::doc).
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

#![expect(
    clippy::needless_doctest_main,
    reason = "The doctest for lib.rs should have a main function"
)]
#![expect(missing_debug_implementations, reason = "Deferred: Noisy")]
#![expect(clippy::allow_attributes_without_reason, reason = "Deferred: Noisy")]
#![expect(
    clippy::shadow_unrelated,
    reason = "Potentially controversial code style"
)]

pub use masonry_core::{
    AsAny, Handled, UnitPoint, assert_render_snapshot, core, cursor_icon, dpi, include_screenshot,
    kurbo, palette, parley, peniko, properties, testing, theme, util, vello, widgets,
};

#[cfg(doc)]
pub use masonry_core::doc;

mod app_driver;
mod convert_winit_event;
mod event_loop_runner;

/// Types needed for running a Masonry app.
pub mod app {
    pub use crate::app_driver::{AppDriver, DriverCtx};
    pub use crate::event_loop_runner::{
        EventLoop, EventLoopBuilder, EventLoopProxy, MasonryState, MasonryUserEvent, WindowState,
        run, run_with,
    };

    pub(crate) use masonry_core::app::try_init_tracing;
    pub use masonry_core::app::{
        RenderRoot, RenderRootOptions, RenderRootSignal, WindowSizePolicy,
    };
}
