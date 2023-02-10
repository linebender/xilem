// This software is licensed under Apache License 2.0 and distributed on an
// "as-is" basis without warranties of any kind. See the LICENSE file for
// details.

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
//! The todo-list example looks like this:
//!
//! ```no_run
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
//!         _env: &Env,
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

#![deny(
    rustdoc::broken_intra_doc_links,
    unsafe_code,
    clippy::trivially_copy_pass_by_ref
)]
#![warn(missing_docs)]
#![warn(unused_imports)]
#![allow(clippy::new_ret_no_self)]
#![allow(clippy::needless_doctest_main)]
#![allow(clippy::should_implement_trait)]
#![allow(clippy::single_match)]
#![allow(clippy::bool_assert_comparison)]
#![cfg_attr(docsrs, feature(doc_cfg))]
#![cfg_attr(not(debug_assertions), allow(unused))]

// TODO - Add logo

pub use druid_shell as shell;
#[doc(inline)]
pub use druid_shell::{kurbo, piet};

#[macro_use]
mod util;

mod action;
mod app_delegate;
mod app_launcher;
mod app_root;
mod bloom;
mod box_constraints;
pub mod command;
mod contexts;
mod data;
pub mod env;
mod event;
pub mod ext_event;
mod mouse;
mod platform;
pub mod promise;
pub mod testing;
pub mod text;
pub mod theme;
pub mod widget;

// TODO
pub mod debug_logger;
pub mod debug_values;

pub use action::Action;
pub use app_delegate::{AppDelegate, DelegateCtx};
pub use app_launcher::AppLauncher;
pub use app_root::{AppRoot, WindowRoot};
pub use box_constraints::BoxConstraints;
pub use command::{Command, Notification, Selector, SingleUse, Target};
pub use contexts::{EventCtx, LayoutCtx, LifeCycleCtx, PaintCtx, WidgetCtx};
pub use data::Data;
pub use druid_shell::Error as PlatformError;
pub use env::{Env, Key, KeyOrValue, Value, ValueType, ValueTypeError};
pub use event::{Event, InternalEvent, InternalLifeCycle, LifeCycle, StatusChange};
pub use kurbo::{Affine, Insets, Point, Rect, Size, Vec2};
pub use mouse::MouseEvent;
pub use piet::{Color, ImageBuf, LinearGradient, RadialGradient, RenderContext, UnitPoint};
pub use platform::{
    MasonryWinHandler, WindowConfig, WindowDescription, WindowId, WindowSizePolicy,
};
pub use text::ArcStr;
pub use util::{AsAny, Handled};
pub use widget::{BackgroundBrush, Widget, WidgetId, WidgetPod, WidgetState};
