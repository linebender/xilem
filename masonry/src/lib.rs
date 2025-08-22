// Copyright 2018 the Xilem Authors and the Druid Authors
// SPDX-License-Identifier: Apache-2.0

//! Masonry is a foundational framework for building GUI libraries in Rust.
//!
//! The developers of Masonry are developing [Xilem], a reactive UI library built on top of Masonry.
//! Masonry's API is geared towards creating GUI libraries; if you are creating an application, we recommend also considering Xilem.
//!
//! Masonry gives you a platform-independent manager, which owns and maintains a widget tree.
//! It also gives you tools to inspect that widget tree at runtime, write unit tests on it, and generally have an easier time debugging and maintaining your app.
//!
//! The framework is not opinionated about what your user-facing abstraction will be: you can implement immediate-mode GUI, the Elm architecture, functional reactive GUI, etc., on top of Masonry.
//!
//! It *is* opinionated about its internals: things like text focus, pointer interactions and accessibility events are often handled in a centralized way.
//!
//! Masonry is built on top of:
//!
//! - [Vello][vello] and [wgpu][vello::wgpu] for 2D graphics.
//! - [Parley][parley] for the text stack.
//! - [AccessKit][accesskit] for plugging into accessibility APIs.
//!
//! Masonry can be used with any windowing library which allows the window content to be rendered using `wgpu`.
//! There are currently two backends for using Masonry to create operating system windows:
//!
//! - [masonry_winit] for most platforms.
//! - `masonry_android_view` for Android. This can currently be found in the [Android View repository](https://github.com/rust-mobile/android-view),
//!   and is not yet generally usable.
//!
//! <!-- TODO: Document that Masonry is a set of baseline widgets and properties built on Masonry core, which can also be used completely independently -->
//!
//! # Example
//!
//! The to-do-list example looks like this, using `masonry_winit` as the backend:
//!
//! ```rust
//! use masonry::core::{ErasedAction, NewWidget, Widget, WidgetId, WidgetPod};
//! use masonry::dpi::LogicalSize;
//! use masonry::properties::types::{Length, AsUnit};
//! use masonry::theme::default_property_set;
//! use masonry::widgets::{Button, ButtonPress, Flex, Label, Portal, TextAction, TextInput};
//! use masonry_winit::app::{AppDriver, DriverCtx, NewWindow, WindowId};
//! use masonry_winit::winit::window::Window;
//!
//! struct Driver {
//!     next_task: String,
//!     window_id: WindowId,
//! }
//!
//! impl AppDriver for Driver {
//!     fn on_action(
//!         &mut self,
//!         window_id: WindowId,
//!         ctx: &mut DriverCtx<'_, '_>,
//!         _widget_id: WidgetId,
//!         action: ErasedAction,
//!     ) {
//!         debug_assert_eq!(window_id, self.window_id, "unknown window");
//!
//!         if action.is::<ButtonPress>() {
//!             ctx.render_root(window_id).edit_root_widget(|mut root| {
//!                 let mut portal = root.downcast::<Portal<Flex>>();
//!                 let mut flex = Portal::child_mut(&mut portal);
//!                 Flex::add_child(&mut flex, Label::new(self.next_task.clone()).with_auto_id());
//!             });
//!         } else if action.is::<TextAction>() {
//!             let action = *action.downcast::<TextAction>().unwrap();
//!             match action {
//!                 TextAction::Changed(new_text) => {
//!                     self.next_task = new_text.clone();
//!                 }
//!                 _ => {}
//!             }
//!         }
//!     }
//! }
//!
//! fn main() {
//!     const WIDGET_SPACING: Length = Length::const_px(5.0);
//!
//!     let main_widget = Portal::new(
//!         Flex::column()
//!             .with_child(NewWidget::new(
//!                 Flex::row()
//!                     .with_flex_child(TextInput::new("").with_auto_id(), 1.0)
//!                     .with_child(
//!                         Button::new(
//!                             Label::new("Add task").with_auto_id()
//!                         ).with_auto_id()
//!                     ),
//!             ))
//!             .with_spacer(WIDGET_SPACING)
//!             .with_auto_id(),
//!     );
//!
//!     let window_size = LogicalSize::new(400.0, 400.0);
//!     let window_attributes = masonry_winit::winit::window::WindowAttributes::default()
//!         .with_title("To-do list")
//!         .with_resizable(true)
//!         .with_min_inner_size(window_size);
//!
//!     let driver = Driver {
//!         next_task: String::new(),
//!         window_id: WindowId::next(),
//!     };
//!     # return;
//!     let event_loop = masonry_winit::app::EventLoop::builder()
//!         .build()
//!         .unwrap();
//!     masonry_winit::app::run_with(
//!         event_loop,
//!         vec![NewWindow::new_with_id(
//!             driver.window_id,
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
//! # Feature flags
//!
//! The following crate [feature flags](https://doc.rust-lang.org/cargo/reference/features.html#dependency-features) are available:
//!
//! - `tracy`: Enables creating output for the [Tracy](https://github.com/wolfpld/tracy) profiler using [`tracing-tracy`][tracing_tracy].
//!   This can be used by installing Tracy and connecting to a Masonry with this feature enabled.
//! - `testing`: Re-exports the test harness from [`masonry_testing`].
//!
//! # Debugging features
//!
//! Masonry apps currently ship with several debugging features built in:
//!
//! - A rudimentary widget inspector - toggled by the F11 key.
//! - A debug mode painting widget layout rectangles - toggled by the F12 key.
//! - Automatic registration of a [tracing] subscriber, which outputs to the console and to a file in the dev profile.
//!
//! [masonry_winit]: https://crates.io/crates/masonry_winit
//! [Xilem]: https://github.com/linebender/xilem/tree/main/xilem
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
// TODO: Remove any items listed as "Deferred"
#![expect(missing_debug_implementations, reason = "Deferred: Noisy")]
#![expect(clippy::cast_possible_truncation, reason = "Deferred: Noisy")]
#![expect(clippy::single_match, reason = "General policy not decided")]
#![expect(
    clippy::needless_doctest_main,
    reason = "Having a main function is a deliberate part of the root doc."
)]

// TODO - Add logo

// TODO - re-add #[doc(hidden)]
pub mod doc;

#[cfg(test)]
mod tests;

pub mod properties;
pub mod theme;
pub mod widgets;

pub use accesskit;
pub use parley::{Alignment as TextAlign, AlignmentOptions as TextAlignOptions};
pub use vello::peniko::color::palette;
pub use vello::{kurbo, peniko};
pub use {dpi, parley, vello};

pub use masonry_core::{app, core, util};
#[cfg(any(feature = "testing", test))]
pub use masonry_testing as testing;
pub use ui_events;
