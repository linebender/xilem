// Copyright 2022 the Xilem Authors
// SPDX-License-Identifier: Apache-2.0

mod box_constraints;
mod button;
mod contexts;
mod core;
//mod layout_observer;
//mod list;
mod linear_layout;
pub mod piet_scene_helpers;
mod raw_event;
mod scroll_view;
mod switch;
mod text;
pub mod tree_structure;
#[allow(clippy::module_inception)]
mod widget;

pub use self::core::{ChangeFlags, Pod};
pub(crate) use self::core::{PodFlags, WidgetState};
pub use box_constraints::BoxConstraints;
pub use button::Button;
pub use contexts::{CxState, EventCx, LayoutCx, LifeCycleCx, PaintCx, UpdateCx};
pub use linear_layout::LinearLayout;
pub use raw_event::{Event, LifeCycle, MouseEvent, PointerCrusher, ScrollDelta, ViewContext};
pub use scroll_view::ScrollView;
pub use switch::Switch;
pub use text::TextWidget;
pub use tree_structure::TreeStructure;
pub use widget::{AnyWidget, Widget};

#[cfg(feature = "taffy")]
mod taffy_layout;
#[cfg(feature = "taffy")]
pub use taffy_layout::TaffyLayout;
