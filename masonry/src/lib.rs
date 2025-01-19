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
//! use masonry::dpi::LogicalSize;
//! use masonry::widget::{Button, Flex, Label, Portal, RootWidget, Textbox, WidgetMut};
//! use masonry::{Action, AppDriver, DriverCtx, WidgetId};
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
//!     masonry::event_loop_runner::run(
//!         masonry::event_loop_runner::EventLoop::with_user_event(),
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

// LINEBENDER LINT SET - lib.rs - v1
// See https://linebender.org/wiki/canonical-lints/
// These lints aren't included in Cargo.toml because they
// shouldn't apply to examples and tests
#![warn(unused_crate_dependencies)]
#![warn(clippy::print_stdout, clippy::print_stderr)]
// END LINEBENDER LINT SET
#![cfg_attr(docsrs, feature(doc_auto_cfg))]
#![cfg_attr(docsrs, feature(doc_cfg))]
#![cfg_attr(
    test,
    expect(
        unused_crate_dependencies,
        reason = "False-positive with dev-dependencies only used in examples"
    )
)]
#![expect(clippy::allow_attributes, reason = "Deferred: Noisy")]
#![expect(clippy::allow_attributes_without_reason, reason = "Deferred: Noisy")]
// TODO: Remove any items listed as "Deferred"
#![expect(clippy::needless_doctest_main, reason = "Deferred: Noisy")]
#![expect(clippy::should_implement_trait, reason = "Deferred: Noisy")]
#![cfg_attr(not(debug_assertions), expect(unused, reason = "Deferred: Noisy"))]
#![expect(let_underscore_drop, reason = "Deferred: Noisy")]
#![expect(missing_debug_implementations, reason = "Deferred: Noisy")]
#![expect(unused_qualifications, reason = "Deferred: Noisy")]
#![expect(single_use_lifetimes, reason = "Deferred: Noisy")]
#![expect(clippy::exhaustive_enums, reason = "Deferred: Noisy")]
#![expect(clippy::match_same_arms, reason = "Deferred: Noisy")]
#![expect(clippy::cast_possible_truncation, reason = "Deferred: Noisy")]
#![expect(clippy::missing_assert_message, reason = "Deferred: Noisy")]
#![expect(clippy::return_self_not_must_use, reason = "Deferred: Noisy")]
#![expect(elided_lifetimes_in_paths, reason = "Deferred: Noisy")]
#![expect(unreachable_pub, reason = "Potentially controversial code style")]
#![expect(
    unnameable_types,
    reason = "Requires lint_reasons rustc feature for exceptions"
)]
#![expect(clippy::todo, reason = "We have a lot of 'real' todos")]
#![expect(clippy::missing_errors_doc, reason = "Can be quite noisy?")]
#![expect(clippy::missing_panics_doc, reason = "Can be quite noisy?")]
#![expect(
    clippy::shadow_unrelated,
    reason = "Potentially controversial code style"
)]
#![expect(clippy::single_match, reason = "General policy not decided")]

// TODO - Add logo

#[macro_use]
mod util;

#[cfg(doc)]
pub mod doc;

mod action;
mod app_driver;
mod box_constraints;
mod contexts;
mod event;
mod paint_scene_helpers;
mod passes;
mod render_root;
mod tracing_backend;

pub mod event_loop_runner;
pub mod testing;
pub mod text;
pub mod theme;
pub mod widget;

pub use cursor_icon;
pub use dpi;
pub use parley;
pub use vello;
pub use vello::kurbo;

pub use cursor_icon::{CursorIcon, ParseError as CursorIconParseError};
pub use kurbo::{Affine, Insets, Point, Rect, Size, Vec2};
pub use parley::layout::Alignment as TextAlignment;
pub use parley::style::FontWeight;
pub use vello::peniko::{color::palette, Color, Gradient};

pub use action::Action;
pub use app_driver::{AppDriver, DriverCtx};
pub use box_constraints::BoxConstraints;
pub use contexts::{
    AccessCtx, ComposeCtx, EventCtx, IsContext, LayoutCtx, MutateCtx, PaintCtx, QueryCtx,
    RawWrapper, RawWrapperMut, RegisterCtx, UpdateCtx,
};
pub use event::{
    AccessEvent, PointerButton, PointerEvent, PointerState, TextEvent, Update, WindowEvent,
    WindowTheme,
};
pub use paint_scene_helpers::UnitPoint;
pub use render_root::{RenderRoot, RenderRootOptions, RenderRootSignal, WindowSizePolicy};
pub use util::{AsAny, Handled};
pub use widget::widget::{AllowRawMut, FromDynWidget, Widget, WidgetId};
pub use widget::WidgetPod;

pub(crate) use widget::WidgetState;
