// Copyright 2024 the Xilem Authors
// SPDX-License-Identifier: Apache-2.0

use tracing::info_span;

use crate::passes::merge_state_up;
use crate::window_root::WindowRoot;
use crate::widget::WidgetMut;
use crate::{MutateCtx, Widget, WidgetId};

pub(crate) fn mutate_widget<R>(
    root: &mut WindowRoot,
    id: WidgetId,
    mutate_fn: impl FnOnce(WidgetMut<'_, Box<dyn Widget>>) -> R,
) -> R {
    let (widget_mut, state_mut) = root.widget_arena.get_pair_mut(id);

    let _span = info_span!("mutate_widget", name = widget_mut.item.short_type_name()).entered();
    // NOTE - parent_widget_state can be None here, because the loop below will merge the
    // state up to the root.
    let root_widget = WidgetMut {
        ctx: MutateCtx {
            global_state: &mut root.global_state,
            parent_widget_state: None,
            widget_state: state_mut.item,
            widget_state_children: state_mut.children,
            widget_children: widget_mut.children,
        },
        widget: widget_mut.item,
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

// TODO - Add link to mutate pass documentation
/// Apply any deferred mutations (created using [`...Ctx::mutate_later`](crate::LayoutCtx::mutate_later)).
pub(crate) fn run_mutate_pass(root: &mut WindowRoot) {
    let callbacks = std::mem::take(&mut root.global_state.mutate_callbacks);
    for callback in callbacks {
        mutate_widget(root, callback.id, callback.callback);
    }
}
