// Copyright 2024 the Xilem Authors
// SPDX-License-Identifier: Apache-2.0

use anymap3::AnyMap;
use tracing::info_span;
use tree_arena::ArenaMut;
use vello::kurbo::Affine;

use crate::app::{RenderRoot, RenderRootState};
use crate::core::{ComposeCtx, Widget, WidgetState};
use crate::passes::{enter_span_if, recurse_on_children};

// --- MARK: RECURSE ---
fn compose_widget(
    global_state: &mut RenderRootState,
    mut widget: ArenaMut<'_, Box<dyn Widget>>,
    mut state: ArenaMut<'_, WidgetState>,
    mut properties: ArenaMut<'_, AnyMap>,
    parent_transformed: bool,
    parent_window_transform: Affine,
) {
    let _span = enter_span_if(
        global_state.trace.compose,
        global_state,
        widget.reborrow(),
        state.reborrow(),
        properties.reborrow(),
    );

    let transformed = parent_transformed || state.item.transform_changed;

    if !transformed && !state.item.needs_compose {
        return;
    }

    // the translation needs to be applied *after* applying the transform, as translation by scrolling should be within the transformed coordinate space. Same is true for the (layout) origin, to behave similar as in CSS.
    let local_translation = state.item.scroll_translation + state.item.origin.to_vec2();

    state.item.window_transform =
        parent_window_transform * state.item.transform.then_translate(local_translation);

    let local_rect = state.item.size.to_rect() + state.item.paint_insets;
    state.item.bounding_rect = state.item.window_transform.transform_rect_bbox(local_rect);

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
    state.item.transform_changed = false;

    let id = state.item.id;
    let parent_transform = state.item.window_transform;
    let parent_state = state.item;
    recurse_on_children(
        id,
        widget.reborrow_mut(),
        state.children,
        properties.children,
        |widget, mut state, properties| {
            compose_widget(
                global_state,
                widget,
                state.reborrow_mut(),
                properties,
                transformed,
                parent_transform,
            );
            let parent_bounding_rect = parent_state.bounding_rect;

            if let Some(child_bounding_rect) = parent_state.clip_child(state.item.bounding_rect) {
                parent_state.bounding_rect = parent_bounding_rect.union(child_bounding_rect);
            }

            parent_state.merge_up(state.item);
        },
    );
}

// --- MARK: ROOT ---
/// See the [passes documentation](../doc/05_pass_system.md#compose-pass).
pub(crate) fn run_compose_pass(root: &mut RenderRoot) {
    let _span = info_span!("compose").entered();

    // If widgets have moved, pointer-related info may be stale.
    // For instance, the "hovered" widget may have moved and no longer be under the pointer.
    if root.root_state().needs_compose {
        root.global_state.needs_pointer_pass = true;
    }

    let (root_widget, root_state, root_properties) = root.widget_arena.get_all_mut(root.root.id());
    compose_widget(
        &mut root.global_state,
        root_widget,
        root_state,
        root_properties,
        false,
        Affine::IDENTITY,
    );
}
