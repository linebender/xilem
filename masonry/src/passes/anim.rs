// Copyright 2024 the Xilem Authors
// SPDX-License-Identifier: Apache-2.0

use tracing::info_span;
use tree_arena::ArenaMut;

use crate::app::{RenderRoot, RenderRootState};
use crate::core::{UpdateCtx, Widget, WidgetState};
use crate::passes::{enter_span_if, recurse_on_children};

// --- MARK: UPDATE ANIM ---
fn update_anim_for_widget(
    global_state: &mut RenderRootState,
    mut widget: ArenaMut<'_, Box<dyn Widget>>,
    mut state: ArenaMut<'_, WidgetState>,
    elapsed_ns: u64,
) {
    let _span = enter_span_if(
        global_state.trace.anim,
        global_state,
        widget.reborrow(),
        state.reborrow(),
    );
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

// TODO - switch anim frames to being about age / an absolute timestamp
// instead of time elapsed.
// (this will help in cases where we want to skip anim frames)

/// Run the animation pass.
///
/// See the [passes documentation](../doc/05_pass_system.md#animation-pass).
pub(crate) fn run_update_anim_pass(root: &mut RenderRoot, elapsed_ns: u64) {
    let _span = info_span!("update_anim").entered();

    let (root_widget, mut root_state) = root.widget_arena.get_pair_mut(root.root.id());
    update_anim_for_widget(
        &mut root.global_state,
        root_widget,
        root_state.reborrow_mut(),
        elapsed_ns,
    );
}
