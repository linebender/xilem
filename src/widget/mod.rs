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

//mod core;
//mod contexts;

mod box_constraints;
mod raw_event;
pub mod tree_structure;

//pub use self::core::{ChangeFlags, Pod};
//pub(crate) use self::core::{PodFlags, WidgetState};
pub use box_constraints::BoxConstraints;
//pub use button::Button;
// pub use contexts::{CxState, EventCx, LayoutCx, LifeCycleCx, PaintCx, UpdateCx};
// pub use linear_layout::LinearLayout;
pub use raw_event::{Event, LifeCycle, MouseEvent, PointerCrusher, ScrollDelta, ViewContext};
// pub use scroll_view::ScrollView;
// pub use switch::Switch;
// pub use text::TextWidget;
pub use tree_structure::TreeStructure;

#[cfg(feature = "taffy")]
mod taffy_layout;
#[cfg(feature = "taffy")]
pub use taffy_layout::TaffyLayout;

use bitflags::bitflags;
bitflags! {
    #[derive(Clone, Copy, Debug, Default, PartialEq, Eq, Hash)]
    #[must_use]
    pub struct ChangeFlags: u8 {
        const UPDATE = 1;
        const LAYOUT = 2;
        const ACCESSIBILITY = 4;
        const PAINT = 8;
        const TREE = 0x10;
        const DESCENDANT_REQUESTED_ACCESSIBILITY = 0x20;
    }
}

impl ChangeFlags {
    // Change flags representing change of tree structure.
    pub fn tree_structure() -> Self {
        ChangeFlags::TREE
    }
}
