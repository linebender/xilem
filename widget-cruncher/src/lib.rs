// Copyright 2018 The Druid Authors.
//
// Licensed under the Apache License, Version 2.0 (the "License");
// you may not use this file except in compliance with the License.
// You may obtain a copy of the License at
//
//     http://www.apache.org/licenses/LICENSE-2.0
//
// Unless required by applicable law or agreed to in writing, software
// distributed under the License is distributed on an "AS IS" BASIS,
// WITHOUT WARRANTIES OR CONDITIONS OF ANY KIND, either express or implied.
// See the License for the specific language governing permissions and
// limitations under the License.

//! Fork of Druid

#![deny(
    broken_intra_doc_links,
    unsafe_code,
    clippy::trivially_copy_pass_by_ref
)]
//#![warn(missing_docs)]
#![allow(clippy::new_ret_no_self, clippy::needless_doctest_main)]
#![cfg_attr(docsrs, feature(doc_cfg))]
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

pub mod action;
pub mod app_delegate;
pub mod app_launcher;
pub mod app_root;
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

// Types from kurbo & piet that are required by public API.
pub use kurbo::{Affine, Insets, Point, Rect, Size, Vec2};
pub use piet::{Color, ImageBuf, LinearGradient, RadialGradient, RenderContext, UnitPoint};

pub use druid_shell::Error as PlatformError;
pub use text::ArcStr;

pub use app_launcher::AppLauncher;
pub use app_root::WindowRoot;

pub use platform::DruidWinHandler;
pub use platform::{WindowConfig, WindowDesc, WindowId, WindowSizePolicy};

pub use box_constraints::BoxConstraints;
pub use command::{Command, Notification, Selector, SingleUse, Target};
pub use contexts::{EventCtx, LayoutCtx, LifeCycleCtx, PaintCtx};
pub use data::Data;
pub use env::{Env, Key, KeyOrValue, Value, ValueType, ValueTypeError};
pub use event::{Event, InternalEvent, InternalLifeCycle, LifeCycle, StatusChange};
pub use mouse::MouseEvent;
pub use util::AsAny;
pub use util::Handled;
pub use widget::{Widget, WidgetId, WidgetPod, WidgetState};
