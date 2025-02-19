// Copyright 2025 the Xilem Authors
// SPDX-License-Identifier: Apache-2.0

//! Basic types and traits Masonry is built on.

mod action;
mod box_constraints;
mod contexts;
mod event;
mod object_fit;
mod properties;
mod text;
#[allow(missing_docs, reason = "TODO")]
mod widget;
mod widget_arena;
#[allow(missing_docs, reason = "TODO")]
mod widget_mut;
#[allow(missing_docs, reason = "TODO")]
mod widget_pod;
mod widget_ref;
mod widget_state;

pub use action::Action;
pub use box_constraints::BoxConstraints;
pub use contexts::{
    AccessCtx, ComposeCtx, EventCtx, IsContext, LayoutCtx, MutateCtx, PaintCtx, QueryCtx,
    RawWrapper, RawWrapperMut, RegisterCtx, UpdateCtx,
};
pub use event::{
    AccessEvent, PointerButton, PointerEvent, PointerState, TextEvent, Update, WindowEvent,
    WindowTheme,
};
pub use object_fit::ObjectFit;
pub use properties::{Properties, PropertiesMut, PropertiesRef, WidgetProperty};
pub use text::{ArcStr, BrushIndex, StyleProperty, StyleSet, render_text};
pub use widget::find_widget_at_pos;
pub use widget::{AllowRawMut, FromDynWidget, Widget, WidgetId};
pub use widget_mut::WidgetMut;
pub use widget_pod::WidgetPod;
pub use widget_ref::WidgetRef;

pub(crate) use text::default_styles;
pub(crate) use widget_arena::WidgetArena;
pub(crate) use widget_pod::CreateWidget;
pub(crate) use widget_state::WidgetState;
