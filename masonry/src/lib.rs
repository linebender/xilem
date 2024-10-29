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
//! # Example
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
//!                 let mut root: WidgetMut<RootWidget<Portal<Flex>>> = ctx.get_root();
//!                 let mut portal = RootWidget::child_mut(&mut root);
//!                 let mut flex = Portal::child_mut(&mut portal);
//!                 Flex::add_child(&mut flex, Label::new(self.next_task.clone()));
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
//! ## Crate feature flags
//!
//! The following feature flags are available:
//!
//! - `tracy`: Enables creating output for the [Tracy](https://github.com/wolfpld/tracy) profiler using [`tracing-tracy`][tracing_tracy].
//!   This can be used by installing Tracy and connecting to a Masonry with this feature enabled.
//!
//! [winit]: https://crates.io/crates/winit
//! [Druid]: https://crates.io/crates/druid
//! [Xilem]: https://crates.io/crates/xilem

// TODO: Remove this once the issues within masonry are fixed. Tracked in https://github.com/linebender/xilem/issues/449
#![allow(rustdoc::broken_intra_doc_links)]
#![deny(clippy::trivially_copy_pass_by_ref)]
// #![deny(rustdoc::broken_intra_doc_links)]
// #![warn(missing_docs)]
#![warn(unused_imports)]
#![warn(clippy::print_stdout, clippy::print_stderr, clippy::dbg_macro)]
#![allow(clippy::needless_doctest_main)]
#![allow(clippy::should_implement_trait)]
#![allow(clippy::single_match)]
#![cfg_attr(docsrs, feature(doc_cfg))]
#![cfg_attr(not(debug_assertions), allow(unused))]
// False-positive with dev-dependencies only used in examples
#![cfg_attr(not(test), warn(unused_crate_dependencies))]

// TODO - Add logo

#[macro_use]
mod util;

#[allow(unused)]
mod debug_logger;
#[allow(unused)]
mod debug_values;

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
mod tree_arena;

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
pub use parley::fontique::Weight as TextWeight;
pub use parley::layout::Alignment as TextAlignment;
pub use vello::peniko::{Color, Gradient};

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
pub use widget::widget::{AllowRawMut, Widget, WidgetId};
pub use widget::WidgetPod;

pub(crate) use widget::WidgetState;
