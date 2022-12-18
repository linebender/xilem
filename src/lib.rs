// This software is licensed under Apache License 2.0 and distributed on an
// "as-is" basis without warranties of any kind. See the LICENSE file for
// details.

//! Fork of Druid

#![deny(
    broken_intra_doc_links,
    unsafe_code,
    clippy::trivially_copy_pass_by_ref
)]
#![warn(missing_docs)]
#![warn(unused_imports)]
#![allow(clippy::new_ret_no_self, clippy::needless_doctest_main)]
#![cfg_attr(docsrs, feature(doc_cfg))]
#![cfg_attr(not(debug_assertions), allow(unused))]
#![doc(
    html_logo_url = "https://raw.githubusercontent.com/linebender/druid/screenshots/images/doc_logo.png"
)]

// Allows to use macros from druid_derive in this crate
extern crate self as druid;
pub use druid_derive::Lens;
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
mod command;
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
pub use app_root::WindowRoot;
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
