// Copyright 2024 the Xilem Authors
// SPDX-License-Identifier: Apache-2.0

use kurbo::{Affine, Point, Rect};
use tracing::info_span;
use tree_arena::ArenaMut;

use crate::app::{RenderRoot, RenderRootState};
use crate::core::{ComposeCtx, PropertyArena, WidgetArenaNode};
use crate::passes::{enter_span_if, recurse_on_children};

/// Returns whether the transform supports box snapping.
///
/// Box snapping is supported when the transform maps local widget axes to device
/// axes without rotation or shear. Scaling, translation, and axis flips are fine.
fn supports_box_snapping(transform: Affine) -> bool {
    let [a, b, c, d, _, _] = transform.as_coeffs();

    // Kurbo affine coefficients represent:
    //
    // x' = a*x + c*y + e
    // y' = b*x + d*y + f
    //
    // The off-diagonal coefficients b and c must be zero. If either is non-zero, x contributes
    // to output y or y contributes to output x. That means the transform mixes axes, as in
    // rotation, shear, or axis swapping, which this snapping path intentionally does not support.
    //
    // The scale coefficients a and d must be non-zero so the transform can be inverted
    // when mapping snapped device edges back to local coordinates.
    //
    // The translation coefficients e and f do not affect whether edges stay axis-aligned,
    // so they are intentionally ignored.
    b == 0. && c == 0. && a != 0. && d != 0.
}

/// Snaps the given `border_box` to device pixel edges.
///
/// If `window_transform` doesn't support snapping then the `border_box` is returned as-is.
fn snap_border_box(border_box: Rect, window_transform: Affine, scale_factor: f64) -> Rect {
    let local_to_device = window_transform.then_scale(scale_factor);
    if !supports_box_snapping(local_to_device) {
        return border_box;
    }

    let device_border_box = local_to_device.transform_rect_bbox(border_box);
    let snapped_device_border_box = Rect::new(
        device_border_box.x0.round(),
        device_border_box.y0.round(),
        device_border_box.x1.round(),
        device_border_box.y1.round(),
    );

    let device_to_local = local_to_device.inverse();
    Rect::from_points(
        device_to_local * Point::new(snapped_device_border_box.x0, snapped_device_border_box.y0),
        device_to_local * Point::new(snapped_device_border_box.x1, snapped_device_border_box.y1),
    )
}

// --- MARK: RECURSE
fn compose_widget(
    global_state: &mut RenderRootState,
    property_arena: &PropertyArena,
    node: ArenaMut<'_, WidgetArenaNode>,
    parent_transformed: bool,
    parent_window_transform: Affine,
) {
    let mut children = node.children;
    let widget = &mut *node.item.widget;
    let state = &mut node.item.state;
    let id = state.id;
    let _span = enter_span_if(global_state.trace.compose, state);

    let transformed = parent_transformed || state.transform_changed;

    if !transformed && !state.needs_compose {
        return;
    }

    // The translation needs to be applied *after* applying the transform,
    // as translation by scrolling should be within the transformed coordinate space.
    // Same is true for the layout border-box origin, to behave similar as in CSS.
    let local_translation = state.scroll_translation + state.layout_origin.to_vec2();

    state.window_transform =
        parent_window_transform * state.transform.then_translate(local_translation);

    let visual_border_box = snap_border_box(
        state.layout_border_box(),
        state.window_transform,
        global_state.scale_factor,
    );
    if state.visual_border_box != visual_border_box {
        state.visual_border_box = visual_border_box;
        // New visual box means that we need to fully redo painting.
        state.request_pre_paint = true;
        state.request_paint = true;
        state.request_post_paint = true;
    }

    let paint_box = state.visual_paint_box();
    state.bounding_box = state.window_transform.transform_rect_bbox(paint_box);

    let mut ctx = ComposeCtx {
        global_state,
        widget_state: state,
        children: children.reborrow_mut(),
        property_arena,
    };
    if ctx.widget_state.request_compose {
        widget.compose(&mut ctx);
    }

    // We need to update the accessibility node's coordinates and repaint it at the new position.
    state.request_accessibility = true;
    state.needs_accessibility = true;
    state.needs_paint = true;

    state.needs_compose = false;
    state.request_compose = false;
    state.transform_changed = false;

    let parent_transform = state.window_transform;
    let parent_state = state;
    recurse_on_children(id, widget, children, |mut node| {
        compose_widget(
            global_state,
            property_arena,
            node.reborrow_mut(),
            transformed,
            parent_transform,
        );
        let parent_bounding_box = parent_state.bounding_box;

        if let Some(child_bounding_box) = parent_state.clip_child(node.item.state.bounding_box) {
            parent_state.bounding_box = parent_bounding_box.union(child_bounding_box);
        }

        parent_state.merge_up(&mut node.item.state);
    });
}

// --- MARK: ROOT
/// See the [passes documentation](crate::doc::pass_system#compose-pass).
pub(crate) fn run_compose_pass(root: &mut RenderRoot) {
    let _span = info_span!("compose").entered();

    // If widgets have moved, pointer-related info may be stale.
    // For instance, the "hovered" widget may have moved and no longer be under the pointer.
    if root.root_state().needs_compose {
        root.global_state.needs_pointer_pass = true;
    }

    let root_node = root.widget_arena.get_node_mut(root.root_id());
    compose_widget(
        &mut root.global_state,
        &root.property_arena,
        root_node,
        false,
        Affine::IDENTITY,
    );
}
