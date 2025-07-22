// Copyright 2025 the Xilem Authors
// SPDX-License-Identifier: Apache-2.0

//! Basic types and traits Masonry is built on.

mod box_constraints;
mod contexts;
mod events;
mod object_fit;
mod properties;
mod text;
mod widget;
mod widget_arena;
mod widget_mut;
mod widget_pod;
mod widget_ref;
mod widget_state;

use anymore::AnyDebug;
pub use box_constraints::BoxConstraints;
pub use contexts::{
    AccessCtx, ComposeCtx, EventCtx, IsContext, LayoutCtx, MutateCtx, PaintCtx, QueryCtx,
    RawWrapper, RawWrapperMut, RegisterCtx, UpdateCtx,
};
pub use events::{
    AccessEvent, Handled, Ime, ResizeDirection, TextEvent, Update, WindowEvent, WindowTheme,
};
pub use object_fit::ObjectFit;
pub use properties::{DefaultProperties, Properties, PropertiesMut, PropertiesRef, Property};
pub use text::{ArcStr, BrushIndex, StyleProperty, StyleSet, render_text};
pub use widget::find_widget_under_pointer;
pub use widget::{AllowRawMut, AsDynWidget, ChildrenIds, FromDynWidget, Widget, WidgetId};
pub use widget_mut::WidgetMut;
pub use widget_pod::{NewWidget, WidgetOptions, WidgetPod};
pub use widget_ref::WidgetRef;

pub use ui_events::keyboard::{KeyboardEvent, Modifiers};
pub use ui_events::pointer::{
    PointerButton, PointerEvent, PointerId, PointerInfo, PointerState, PointerType, PointerUpdate,
};
pub use ui_events::{ScrollDelta, keyboard, pointer};

pub(crate) use widget_arena::{WidgetArena, WidgetArenaMut, WidgetArenaRef};
pub(crate) use widget_state::WidgetState;

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
