// Copyright 2024 the Xilem Authors
// SPDX-License-Identifier: Apache-2.0

use tracing::info_span;
use tree_arena::ArenaMut;

use crate::app::RenderRoot;
use crate::core::{MutateCtx, PropertiesMut, Widget, WidgetId, WidgetMut, WidgetState};
use crate::passes::merge_state_up;

pub(crate) fn mutate_widget<R>(
    root: &mut RenderRoot,
    id: WidgetId,
    mutate_fn: impl FnOnce(WidgetMut<'_, dyn Widget>) -> R,
) -> R {
    let (mut widget_mut, mut state_mut, mut properties_mut) = root.widget_arena.get_all_mut(id);

    let _span = info_span!("mutate_widget", name = widget_mut.item.short_type_name()).entered();

    let root_widget = WidgetMut {
        ctx: MutateCtx {
            global_state: &mut root.global_state,
            widget_state: state_mut.item,
            widget_state_children: state_mut.children.reborrow_mut(),
            widget_children: widget_mut.children.reborrow_mut(),
            properties: PropertiesMut {
                map: properties_mut.item,
                default_map: root
                    .default_properties
                    .for_widget(widget_mut.item.type_id()),
            },
            properties_children: properties_mut.children.reborrow_mut(),
        },
        widget: &mut **widget_mut.item,
    };
    root_widget.ctx.widget_state.needs_update_flags = true;

    let result = mutate_fn(root_widget);

    update_flags(widget_mut, state_mut);

    // Merge all state changes up to the root.
    let mut current_id = Some(id);
    while let Some(id) = current_id {
        let parent_id = root.widget_arena.parent_of(id);
        merge_state_up(&mut root.widget_arena, id);
        current_id = parent_id;
    }

    result
}

fn update_flags(mut widget: ArenaMut<'_, Box<dyn Widget>>, mut state: ArenaMut<'_, WidgetState>) {
    if !state.item.needs_update_flags {
        return;
    }
    state.item.needs_update_flags = false;

    let parent_state = state.item;
    for child_id in widget.item.children_ids() {
        let widget = widget.children.item_mut(child_id);
        let state = state.children.item_mut(child_id);

        if let (Some(widget), Some(mut state)) = (widget, state) {
            update_flags(widget, state.reborrow_mut());
            parent_state.merge_up(state.item);
        }
    }
}

/// Apply any deferred mutations (created using [`...Ctx::mutate_later`]
///
/// See the [passes documentation](../doc/05_pass_system.md#the-mutate-pass).
pub(crate) fn run_mutate_pass(root: &mut RenderRoot) {
    let callbacks = std::mem::take(&mut root.global_state.mutate_callbacks);
    for callback in callbacks {
        mutate_widget(root, callback.id, callback.callback);
    }
}
