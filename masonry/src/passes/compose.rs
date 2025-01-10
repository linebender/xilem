// Copyright 2024 the Xilem Authors
// SPDX-License-Identifier: Apache-2.0

use tracing::info_span;
use tree_arena::ArenaMut;
use vello::kurbo::Vec2;

use crate::passes::{enter_span_if, recurse_on_children};
use crate::render_root::{RenderRoot, RenderRootState};
use crate::{ComposeCtx, Widget, WidgetState};

// --- MARK: RECURSE ---
fn compose_widget(
    global_state: &mut RenderRootState,
    mut widget: ArenaMut<'_, Box<dyn Widget>>,
    mut state: ArenaMut<'_, WidgetState>,
    parent_moved: bool,
    parent_translation: Vec2,
) {
    let _span = enter_span_if(
        global_state.trace.compose,
        global_state,
        widget.reborrow(),
        state.reborrow(),
    );

    let moved = parent_moved || state.item.translation_changed;
    let translation = parent_translation + state.item.translation + state.item.origin.to_vec2();
    state.item.window_origin = translation.to_point();

    if !parent_moved && !state.item.translation_changed && !state.item.needs_compose {
        return;
    }

    let mut ctx = ComposeCtx {
        global_state,
        widget_state: state.item,
        widget_state_children: state.children.reborrow_mut(),
        widget_children: widget.children.reborrow_mut(),
    };
    if ctx.widget_state.request_compose {
        widget.item.compose(&mut ctx);
    }

    // We need to update the accessibility node's coordinates and repaint it at the new position.
    state.item.request_accessibility = true;
    state.item.needs_accessibility = true;
    state.item.needs_paint = true;

    state.item.needs_compose = false;
    state.item.request_compose = false;
    state.item.translation_changed = false;

    let id = state.item.id;
    let parent_state = state.item;
    recurse_on_children(
        id,
        widget.reborrow_mut(),
        state.children,
        |widget, mut state| {
            compose_widget(
                global_state,
                widget,
                state.reborrow_mut(),
                moved,
                translation,
            );
            parent_state.merge_up(state.item);
        },
    );
}

// --- MARK: ROOT ---
pub(crate) fn run_compose_pass(root: &mut RenderRoot) {
    let _span = info_span!("compose").entered();

    // If widgets have moved, pointer-related info may be stale.
    // For instance, the "hovered" widget may have moved and no longer be under the pointer.
    if root.root_state().needs_compose {
        root.global_state.needs_pointer_pass = true;
    }

    let (root_widget, root_state) = root.widget_arena.get_pair_mut(root.root.id());
    compose_widget(
        &mut root.global_state,
        root_widget,
        root_state,
        false,
        Vec2::ZERO,
    );
}
