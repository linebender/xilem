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

#![allow(unused_imports)]
#![allow(unused_variables)]
#![allow(unused_mut)]
#![allow(dead_code)]

// Allows to use macros from druid_derive in this crate
extern crate self as druid;
pub use druid_derive::Lens;

#[doc(inline)]
pub use druid_shell::{kurbo, piet};

#[macro_use]
mod util;

mod window_handling;
mod bloom;
mod box_constraints;
mod command;
mod contexts;
mod core;
mod data;
pub mod env;
mod event;
mod mouse;
pub mod scroll_component;
pub mod text;
pub mod theme;
pub mod widget;
pub mod app_launcher;
pub mod app_root;

// Types from kurbo & piet that are required by public API.
pub use kurbo::{Affine, Insets, Point, Rect, Size, Vec2};
pub use piet::{Color, ImageBuf, LinearGradient, RadialGradient, RenderContext, UnitPoint};

// these are the types from druid_shell that we expose; others we only use internally.
#[cfg(feature = "image")]
pub use druid_shell::image;
pub use druid_shell::keyboard_types;
pub use druid_shell::{
    Application, Clipboard, ClipboardFormat, Code, Cursor, CursorDesc, Error as PlatformError,
    FileInfo, FileSpec, FormatId, HotKey, KbKey, KeyEvent, Location, Modifiers, Monitor,
    MouseButton, MouseButtons, RawMods, Region, Scalable, Scale, Screen, SysMods, TimerToken,
    WindowHandle, WindowLevel, WindowState,
};

#[cfg(feature = "raw-win-handle")]
pub use druid_shell::raw_window_handle::{HasRawWindowHandle, RawWindowHandle};

pub use crate::core::{WidgetPod, WidgetState};
pub use window_handling::window_description::{WindowId, WindowConfig, WindowDesc, WindowSizePolicy};
pub use app_launcher::AppLauncher;
pub use box_constraints::BoxConstraints;
pub use command::{Selector, SingleUse, Target};
pub use contexts::{EventCtx, LayoutCtx, LifeCycleCtx, PaintCtx, UpdateCtx};
pub use data::Data;
pub use env::{Env, Key, KeyOrValue, Value, ValueType, ValueTypeError};
pub use event::{Event, InternalEvent, InternalLifeCycle, LifeCycle};
pub use mouse::MouseEvent;
pub use util::Handled;
pub use widget::{Widget, WidgetId};
pub use window_handling::win_handler::DruidHandler;
pub use app_root::{WindowRoot};

#[deprecated(since = "0.8.0", note = "import from druid::text module instead")]
pub use text::{ArcStr, FontDescriptor};

/// The meaning (mapped value) of a keypress.
///
/// Note that in previous versions, the `KeyCode` field referred to the
/// physical position of the key, rather than the mapped value. In most
/// cases, applications should dispatch based on the value instead. This
/// alias is provided to make that transition easy, but in any case make
/// an explicit choice whether to use meaning or physical location and
/// use the appropriate type.
#[deprecated(since = "0.7.0", note = "Use KbKey instead")]
pub type KeyCode = KbKey;

#[deprecated(since = "0.7.0", note = "Use Modifiers instead")]
/// See [`Modifiers`](struct.Modifiers.html).
pub type KeyModifiers = Modifiers;
