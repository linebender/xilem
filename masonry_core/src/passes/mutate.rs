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
    let (item, children) = root.widget_arena.get_mut(root.root.id());

    let _span = info_span!("mutate_widget", name = item.widget.short_type_name()).entered();
    // NOTE - we can set parent_widget_state to None here, because the loop below will merge the
    // states up to the root.
    let root_widget = WidgetMut {
        ctx: MutateCtx {
            global_state: &mut root.global_state,
            parent_widget_state: None,
            widget_state: item.state,
            properties: PropertiesMut {
                map: item.properties,
                default_map: root.default_properties.for_widget(item.widget.type_id()),
            },
            children,
        },
        widget: &mut **item.widget,
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

/// Apply any deferred mutations (created using [`...Ctx::mutate_later`]
///
/// See the [passes documentation](../doc/05_pass_system.md#the-mutate-pass).
pub(crate) fn run_mutate_pass(root: &mut RenderRoot) {
    let callbacks = std::mem::take(&mut root.global_state.mutate_callbacks);
    for callback in callbacks {
        mutate_widget(root, callback.id, callback.callback);
    }
}
