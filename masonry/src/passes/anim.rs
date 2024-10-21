// Copyright 2024 the Xilem Authors
// SPDX-License-Identifier: Apache-2.0

use tracing::info_span;

use crate::passes::recurse_on_children;
use crate::tree_arena::ArenaMut;
use crate::window_root::{WindowRoot, WindowRootState};
use crate::{UpdateCtx, Widget, WidgetState};

// --- MARK: UPDATE ANIM ---
fn update_anim_for_widget(
    global_state: &mut WindowRootState,
    mut widget: ArenaMut<'_, Box<dyn Widget>>,
    mut state: ArenaMut<'_, WidgetState>,
    elapsed_ns: u64,
) {
    let _span = global_state
        .trace
        .anim
        .then(|| widget.item.make_trace_span().entered());

    if !state.item.needs_anim {
        return;
    }
    state.item.needs_anim = false;

    // Most passes reset their `needs` and `request` flags after the call to
    // the widget method, but it's valid and expected for `request_anim` to be
    // set in response to `AnimFrame`.
    if state.item.request_anim {
        state.item.request_anim = false;
        let mut ctx = UpdateCtx {
            global_state,
            widget_state: state.item,
            widget_state_children: state.children.reborrow_mut(),
            widget_children: widget.children.reborrow_mut(),
        };
        widget.item.on_anim_frame(&mut ctx, elapsed_ns);
    }

    let id = state.item.id;
    let parent_state = state.item;
    recurse_on_children(
        id,
        widget.reborrow_mut(),
        state.children,
        |widget, mut state| {
            update_anim_for_widget(global_state, widget, state.reborrow_mut(), elapsed_ns);
            parent_state.merge_up(state.item);
        },
    );
}

/// Run the animation pass.
pub(crate) fn run_update_anim_pass(root: &mut WindowRoot, elapsed_ns: u64) {
    let _span = info_span!("update_anim").entered();

    let (root_widget, mut root_state) = root.widget_arena.get_pair_mut(root.root.id());
    update_anim_for_widget(
        &mut root.global_state,
        root_widget,
        root_state.reborrow_mut(),
        elapsed_ns,
    );
}
