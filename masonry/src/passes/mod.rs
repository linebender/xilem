// Copyright 2024 the Xilem Authors
// SPDX-License-Identifier: Apache-2.0

use crate::tree_arena::{ArenaMut, ArenaMutChildren};
use crate::widget::WidgetArena;
use crate::{Widget, WidgetId, WidgetState};

pub mod accessibility;
pub mod compose;
pub mod event;
pub mod mutate;
pub mod paint;
pub mod update;

pub(crate) fn recurse_on_children(
    id: WidgetId,
    mut widget: ArenaMut<'_, Box<dyn Widget>>,
    mut state: ArenaMutChildren<'_, WidgetState>,
    mut callback: impl FnMut(ArenaMut<'_, Box<dyn Widget>>, ArenaMut<'_, WidgetState>),
) {
    let parent_name = widget.item.short_type_name();
    let parent_id = id;

    for child_id in widget.item.children_ids() {
        let widget = widget
            .children
            .get_child_mut(child_id.to_raw())
            .unwrap_or_else(|| {
                panic!(
                    "Error in '{}' #{}: cannot find child #{} returned by children_ids()",
                    parent_name,
                    parent_id.to_raw(),
                    child_id.to_raw()
                )
            });
        let state = state.get_child_mut(child_id.to_raw()).unwrap_or_else(|| {
            panic!(
                "Error in '{}' #{}: cannot find child #{} returned by children_ids()",
                parent_name,
                parent_id.to_raw(),
                child_id.to_raw()
            )
        });

        callback(widget, state);
    }
}

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
