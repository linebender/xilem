// Copyright 2024 the Xilem Authors
// SPDX-License-Identifier: Apache-2.0

use tracing::info_span;

use crate::{
    render_root::RenderRoot,
    tree_arena::{ArenaMut, ArenaMutChildren},
    widget::{WidgetArena, WidgetMut},
    MutateCtx, Widget, WidgetId, WidgetState,
};

#[allow(unused)]
// References shared by all passes
struct PassCtx<'a> {
    pub(crate) widget_state: ArenaMut<'a, WidgetState>,
    pub(crate) widget_children: ArenaMutChildren<'a, Box<dyn Widget>>,
}

impl<'a> PassCtx<'a> {
    fn parent(&self) -> Option<WidgetId> {
        let parent_id = self.widget_state.parent_id?;
        let parent_id = parent_id.try_into().unwrap();
        Some(WidgetId(parent_id))
    }
}

// TODO - Merge copy-pasted code
fn merge_state_up(arena: &mut WidgetArena, widget_id: WidgetId, root_state: &mut WidgetState) {
    let parent_id = get_widget_mut(arena, widget_id).1.parent();

    let Some(parent_id) = parent_id else {
        // We've reached the root
        let child_state_mut = arena.widget_states.find_mut(widget_id.to_raw()).unwrap();
        root_state.merge_up(child_state_mut.item);
        return;
    };

    let mut parent_state_mut = arena.widget_states.find_mut(parent_id.to_raw()).unwrap();
    let child_state_mut = parent_state_mut
        .children
        .get_child_mut(widget_id.to_raw())
        .unwrap();

    parent_state_mut.item.merge_up(child_state_mut.item);
}

fn get_widget_mut(arena: &mut WidgetArena, id: WidgetId) -> (&mut dyn Widget, PassCtx<'_>) {
    let state_mut = arena
        .widget_states
        .find_mut(id.to_raw())
        .expect("widget state not found in arena");
    let widget_mut = arena
        .widgets
        .find_mut(id.to_raw())
        .expect("widget not found in arena");

    // Box<dyn Widget> -> &dyn Widget
    // Without this step, the type of `WidgetRef::widget` would be
    // `&Box<dyn Widget> as &dyn Widget`, which would be an additional layer
    // of indirection.
    let widget = widget_mut.item;
    let widget: &mut dyn Widget = &mut **widget;

    (
        widget,
        PassCtx {
            widget_state: state_mut,
            widget_children: widget_mut.children,
        },
    )
}

pub(crate) fn mutate_widget(
    root: &mut RenderRoot,
    root_state: &mut WidgetState,
    id: WidgetId,
    mutate_fn: impl FnOnce(WidgetMut<'_, Box<dyn Widget>>),
) {
    let mut fake_widget_state =
        WidgetState::new(root.root.id(), Some(root.get_kurbo_size()), "<root>");

    let state_mut = root
        .widget_arena
        .widget_states
        .find_mut(id.to_raw())
        .expect("widget state not found in arena");
    let widget_mut = root
        .widget_arena
        .widgets
        .find_mut(id.to_raw())
        .expect("widget not found in arena");

    let _span = info_span!("mutate_widget").entered();
    let root_widget = WidgetMut {
        ctx: MutateCtx {
            global_state: &mut root.state,
            parent_widget_state: &mut fake_widget_state,
            widget_state: state_mut.item,
            widget_state_children: state_mut.children,
            widget_children: widget_mut.children,
        },
        widget: widget_mut.item,
        is_reborrow: false,
    };

    mutate_fn(root_widget);

    let mut current_id = Some(id);
    while let Some(id) = current_id {
        let (_widget, pass_ctx) = get_widget_mut(&mut root.widget_arena, id);
        let parent_id = pass_ctx.parent();

        merge_state_up(&mut root.widget_arena, id, root_state);
        current_id = parent_id;
    }
}

pub(crate) fn run_mutate_pass(root: &mut RenderRoot, root_state: &mut WidgetState) {
    // TODO - Factor out into a "pre-event" function?
    // root.state.next_focused_widget = root.state.focused_widget;

    let callbacks = std::mem::take(&mut root.state.mutate_callbacks);
    for callback in callbacks {
        mutate_widget(root, root_state, callback.id, callback.callback);
    }

    // root.post_event_processing(&mut root_state);
}
