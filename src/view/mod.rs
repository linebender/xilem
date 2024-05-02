// Copyright 2022 the Xilem Authors and the Druid Authors
// SPDX-License-Identifier: Apache-2.0

// mod async_list;
mod button;
// mod layout_observer;
// mod list;
mod scroll_view;
mod text;
// mod use_state;
mod linear_layout;
mod list;
mod switch;
mod tree_structure_tracking;
#[allow(clippy::module_inception)]
mod view;

pub use xilem_core::{Id, IdPath, VecSplice};

pub use button::button;
pub use linear_layout::{h_stack, v_stack, LinearLayout};
pub use list::{list, List};
pub use scroll_view::{scroll_view, ScrollView};
pub use switch::switch;
pub use tree_structure_tracking::TreeStructureSplice;
pub use view::{Adapt, AdaptState, Cx, ElementsSplice, Memoize, View, ViewMarker, ViewSequence};

#[cfg(feature = "taffy")]
mod taffy_layout;
#[cfg(feature = "taffy")]
pub use taffy_layout::{div, flex_column, flex_row, grid, TaffyLayout};
