// Copyright 2024 the Xilem Authors
// SPDX-License-Identifier: Apache-2.0

use tracing::info_span;
use vello::kurbo::Affine;

use crate::app::{RenderRoot, RenderRootState};
use crate::core::{ComposeCtx, DefaultProperties, WidgetArenaMut, WidgetItemMut};
use crate::passes::enter_span_if;

// --- MARK: RECURSE
fn compose_widget(
    global_state: &mut RenderRootState,
    default_properties: &DefaultProperties,
    item: WidgetItemMut<'_>,
    mut children: WidgetArenaMut<'_>,
    parent_transformed: bool,
    parent_window_transform: Affine,
) {
    let _span = enter_span_if(global_state.trace.compose, &**item.widget, item.state.id);

    let transformed = parent_transformed || item.state.transform_changed;

    if !transformed && !item.state.needs_compose {
        return;
    }

    // the translation needs to be applied *after* applying the transform, as translation by scrolling should be within the transformed coordinate space. Same is true for the (layout) origin, to behave similar as in CSS.
    let local_translation = item.state.scroll_translation + item.state.origin.to_vec2();

    item.state.window_transform =
        parent_window_transform * item.state.transform.then_translate(local_translation);

    let local_rect = item.state.size.to_rect() + item.state.paint_insets;
    item.state.bounding_rect = item.state.window_transform.transform_rect_bbox(local_rect);

    let mut ctx = ComposeCtx {
        global_state,
        widget_state: item.state,
        widget_state_children: children.state_children.reborrow_mut(),
        widget_children: children.widget_children.reborrow_mut(),
    };
    if ctx.widget_state.request_compose {
        item.widget.compose(&mut ctx);
    }

    // We need to update the accessibility node's coordinates and repaint it at the new position.
    item.state.request_accessibility = true;
    item.state.needs_accessibility = true;
    item.state.needs_paint = true;

    item.state.needs_compose = false;
    item.state.request_compose = false;
    item.state.transform_changed = false;

    let id = item.state.id;
    let parent_transform = item.state.window_transform;
    let parent_state = item.state;
    crate::passes::recurse_on_children2(id, &**item.widget, children, |mut item, children| {
        compose_widget(
            global_state,
            default_properties,
            item.reborrow_mut(),
            children,
            transformed,
            parent_transform,
        );
        let parent_bounding_rect = parent_state.bounding_rect;

        if let Some(child_bounding_rect) = parent_state.clip_child(item.state.bounding_rect) {
            parent_state.bounding_rect = parent_bounding_rect.union(child_bounding_rect);
        }

        parent_state.merge_up(item.state);
    });
}

// --- MARK: ROOT
/// See the [passes documentation](../doc/05_pass_system.md#compose-pass).
pub(crate) fn run_compose_pass(root: &mut RenderRoot) {
    let _span = info_span!("compose").entered();

    // If widgets have moved, pointer-related info may be stale.
    // For instance, the "hovered" widget may have moved and no longer be under the pointer.
    if root.root_state().needs_compose {
        root.global_state.needs_pointer_pass = true;
    }

    let (root_item, root_children) = root.widget_arena.get_mut(root.root.id());
    compose_widget(
        &mut root.global_state,
        &root.default_properties,
        root_item,
        root_children,
        false,
        Affine::IDENTITY,
    );
}
