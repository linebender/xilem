// Copyright 2025 the Xilem Authors
// SPDX-License-Identifier: Apache-2.0

//! Basic types and traits Masonry is built on.

mod contexts;
mod default_properties;
mod events;
mod layer;
mod properties_mut;
mod properties_ref;
mod property;
mod property_set;
mod text;
mod widget;
mod widget_arena;
mod widget_mut;
mod widget_paint;
mod widget_pod;
mod widget_ref;
mod widget_state;
mod widget_tag;

pub use contexts::*;
pub use default_properties::*;
pub use events::*;
pub use layer::*;
pub use properties_mut::*;
pub use properties_ref::*;
pub use property::*;
pub use property_set::*;
pub use text::*;
pub use widget::*;
pub use widget_mut::*;
pub use widget_paint::*;
pub use widget_pod::*;
pub use widget_ref::*;
pub use widget_tag::*;

pub use cursor_icon::CursorIcon;
pub use ui_events::keyboard::{KeyboardEvent, Modifiers};
pub use ui_events::pointer::{
    PointerButton, PointerButtonEvent, PointerEvent, PointerGesture, PointerGestureEvent,
    PointerId, PointerInfo, PointerScrollEvent, PointerState, PointerType, PointerUpdate,
};
pub use ui_events::{ScrollDelta, keyboard, pointer};

pub(crate) use widget_arena::*;
pub(crate) use widget_state::*;

use anymore::AnyDebug;

/// Actions are emitted by Masonry widgets when a user input needs to be handled by the application.
///
/// The concrete action type can be accessed from this type using [`downcast`](anymore::AnyDebug#method.downcast-1).
// N.b. We would like to use a true intra-doc link here, but it's not feasible to do so to `dyn Trait` items.
// see https://github.com/rust-lang/rust/issues/74563
///
/// Widget implementation can create actions using the [`submit_action`](EventCtx::submit_action) method
/// on context types. In Masonry Winit, they are passed to the application through the `on_action` method
/// on `AppDriver`.
///
/// In tests, you can access these using the `pop_action` method on `TestHarness`.
pub type ErasedAction = Box<dyn AnyDebug + Send>;

/// Empty type to be used as the `Widget::Action` associated type for widgets which don't emit actions.
#[derive(Debug)]
pub enum NoAction {}
