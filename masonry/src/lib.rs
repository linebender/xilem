// Copyright 2018 the Xilem Authors and the Druid Authors
// SPDX-License-Identifier: Apache-2.0

//! A framework that aims to provide the foundation for Rust GUI libraries.
//!
//! Masonry gives you a platform to create windows (using Glazier as a backend) each with a tree of widgets. It also gives you tools to inspect that widget tree at runtime, write unit tests on it, and generally have an easier time debugging and maintaining your app.
//!
//! The framework is not opinionated about what your user-facing abstraction will be: you can implement immediate-mode GUI, the Elm architecture, functional reactive GUI, etc, on top of Masonry.
//!
//! This project was originally was originally a fork of Druid that emerged from discussions I had with Raph Levien and Colin Rofls about what it would look like to turn Druid into a foundational library.
//!
//! ## Example
//!
//! **(TODO: FIX THIS EXAMPLE)**
//! The todo-list example looks like this:
//!
//! ```ignore
//! use masonry::widget::{prelude::*, TextBox};
//! use masonry::widget::{Button, Flex, Label, Portal, WidgetMut};
//! use masonry::Action;
//! use masonry::{AppDelegate, AppLauncher, DelegateCtx, WindowDescription, WindowId};
//!
//! const VERTICAL_WIDGET_SPACING: f64 = 20.0;
//!
//! struct Delegate {
//!     next_task: String,
//! }
//!
//! impl AppDelegate for Delegate {
//!     fn on_action(
//!         &mut self,
//!         ctx: &mut DelegateCtx,
//!         _window_id: WindowId,
//!         _widget_id: WidgetId,
//!         action: Action,
//!     ) {
//!         match action {
//!             Action::ButtonPressed => {
//!                 let mut root: WidgetMut<Portal<Flex>> = ctx.get_root();
//!                 let mut flex = root.child_mut();
//!                 flex.add_child(Label::new(self.next_task.clone()));
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
//!     // The main button with some space below, all inside a scrollable area.
//!     let root_widget = Portal::new(
//!         Flex::column()
//!             .with_child(
//!                 Flex::row()
//!                     .with_child(TextBox::new(""))
//!                     .with_child(Button::new("Add task")),
//!             )
//!             .with_spacer(VERTICAL_WIDGET_SPACING),
//!     );
//!
//!     let main_window = WindowDescription::new(root_widget)
//!         .title("To-do list")
//!         .window_size((400.0, 400.0));
//!
//!     AppLauncher::with_window(main_window)
//!         .with_delegate(Delegate {
//!             next_task: String::new(),
//!         })
//!         .log_to_console()
//!         .launch()
//!         .expect("Failed to launch application");
//! }
//! ```

#![deny(unsafe_code, clippy::trivially_copy_pass_by_ref)]
// #![deny(rustdoc::broken_intra_doc_links)]
// #![warn(missing_docs)]
#![warn(unused_imports)]
#![allow(clippy::should_implement_trait)]
#![allow(clippy::single_match)]
#![cfg_attr(docsrs, feature(doc_cfg))]
#![cfg_attr(not(debug_assertions), allow(unused))]

// TODO - Add logo

pub use cursor_icon::{CursorIcon, ParseError as CursorIconParseError};
pub use dpi;
pub use kurbo;
pub use parley;
pub use vello;

#[macro_use]
mod util;

mod action;
mod bloom;
mod box_constraints;
mod contexts;
mod event;
pub mod paint_scene_helpers;
pub mod promise;
pub mod render_root;
pub mod testing;
// mod text;
pub mod text_helpers;
pub mod theme;
pub mod widget;

// TODO
pub mod app_driver;
pub mod debug_logger;
pub mod debug_values;
pub mod event_loop_runner;
pub mod text2;

pub use action::Action;
pub use box_constraints::BoxConstraints;
pub use contexts::{AccessCtx, EventCtx, LayoutCtx, LifeCycleCtx, PaintCtx, WidgetCtx};
pub use event::{
    AccessEvent, InternalLifeCycle, LifeCycle, PointerEvent, PointerState, StatusChange, TextEvent,
    WindowEvent, WindowTheme,
};
pub use kurbo::{Affine, Insets, Point, Rect, Size, Vec2};
pub use parley::layout::Alignment as TextAlignment;
pub use util::{AsAny, Handled};
pub use vello::peniko::{Color, Gradient};
pub use widget::{BackgroundBrush, Widget, WidgetId, WidgetPod, WidgetState};

pub use text_helpers::ArcStr;
