// Copyright 2024 the Xilem Authors
// SPDX-License-Identifier: Apache-2.0

use tracing::info_span;

use crate::app::RenderRoot;
use crate::core::{MutateCtx, PropertiesMut, Widget, WidgetId, WidgetMut};
use crate::passes::merge_state_up;

pub(crate) fn mutate_widget<R>(
    root: &mut RenderRoot,
    id: WidgetId,
    mutate_fn: impl FnOnce(WidgetMut<'_, dyn Widget>) -> R,
) -> R {
    // TODO - This panics if id can't be found.
    // Should it return Option instead?
    let node = root.widget_arena.get_node_mut(id);
    let children = node.children;
    let widget = &mut *node.item.widget;
    let state = &mut node.item.state;
    let properties = &mut node.item.properties;
    let changed_properties = &mut node.item.changed_properties;
    let id = state.id;

    let _span = info_span!("mutate_widget", name = widget.short_type_name()).entered();

    changed_properties.clear();

    // NOTE - we can set parent_widget_state to None here, because the loop below will merge the
    // states up to the root.

    let root_widget = WidgetMut {
        ctx: MutateCtx {
            global_state: &mut root.global_state,
            parent_widget_state: None,
            widget_state: state,
            properties: PropertiesMut {
                set: properties,
                default_map: root.default_properties.for_widget(widget.type_id()),
            },
            changed_properties,
            children,
            default_properties: &root.default_properties,
        },
        widget,
    };

    let result = mutate_fn(root_widget);

    // Merge all state changes up to the root.
    let mut current_id = Some(id);
    while let Some(id) = current_id {
        let parent_id = root.widget_arena.parent_of(id);
        merge_state_up(&mut root.widget_arena, id);
        current_id = parent_id;
    }

    result
}

/// Apply any deferred mutations (created using `...Ctx::mutate_later`)
///
/// See the [passes documentation](crate::doc::pass_system#the-mutate-pass).
pub(crate) fn run_mutate_pass(root: &mut RenderRoot) {
    let callbacks = std::mem::take(&mut root.global_state.mutate_callbacks);
    for callback in callbacks {
        // Skip callbacks whose target was removed since they were emitted.
        if !root.widget_arena.has(callback.id) {
            continue;
        }
        mutate_widget(root, callback.id, callback.callback);
    }
}
