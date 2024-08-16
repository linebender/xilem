// Copyright 2024 the Xilem Authors
// SPDX-License-Identifier: Apache-2.0

use crate::widget::WidgetArena;
use crate::WidgetId;

pub mod compose;
pub mod event;
pub mod mutate;
pub mod update;

pub(crate) fn merge_state_up(arena: &mut WidgetArena, widget_id: WidgetId) {
    let parent_id = arena.parent_of(widget_id);

    let Some(parent_id) = parent_id else {
        // We've reached the root
        return;
    };

    let mut parent_state_mut = arena.widget_states.find_mut(parent_id.to_raw()).unwrap();
    let child_state_mut = parent_state_mut
        .children
        .get_child_mut(widget_id.to_raw())
        .unwrap();

    parent_state_mut.item.merge_up(child_state_mut.item);
}
