// Copyright 2022 The Druid Authors.
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

mod box_constraints;
mod button;
mod contexts;
mod core;
//mod layout_observer;
//mod list;
mod linear_layout;
pub mod piet_scene_helpers;
mod raw_event;
mod switch;
//mod scroll_view;
mod text;
#[allow(clippy::module_inception)]
mod widget;

pub use self::core::{ChangeFlags, Pod};
pub(crate) use self::core::{PodFlags, WidgetState};
pub use box_constraints::BoxConstraints;
pub use button::Button;
pub use contexts::{AccessCx, CxState, EventCx, LayoutCx, LifeCycleCx, PaintCx, UpdateCx};
pub use linear_layout::LinearLayout;
pub use raw_event::{Event, LifeCycle, MouseEvent, PointerCrusher, ViewContext};
pub use switch::Switch;
pub use text::TextWidget;
pub use widget::{AnyWidget, Widget};

#[cfg(feature = "taffy")]
mod taffy_layout;
#[cfg(feature = "taffy")]
pub use taffy_layout::TaffyLayout;
