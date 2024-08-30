// Copyright 2024 the Xilem Authors
// SPDX-License-Identifier: Apache-2.0

//! The layout pass, which provides the size and position of each widget
//! before any translations applied in [`compose`](crate::passes::compose).
//! Most of the logic for this pass happens in [`Widget::layout`] implementations.

use tracing::{info_span, trace, warn};
use vello::kurbo::{Point, Rect, Size};

use crate::render_root::RenderRoot;
use crate::widget::WidgetState;
use crate::{BoxConstraints, LayoutCtx, Widget, WidgetPod};

// TODO - negative rects?
/// Return `true` if all of `smaller` is within `larger`.
fn rect_contains(larger: &Rect, smaller: &Rect) -> bool {
    smaller.x0 >= larger.x0
        && smaller.x1 <= larger.x1
        && smaller.y0 >= larger.y0
        && smaller.y1 <= larger.y1
}

pub(crate) fn run_layout_inner<W: Widget>(
    parent_ctx: &mut LayoutCtx<'_>,
    pod: &mut WidgetPod<W>,
    bc: &BoxConstraints,
) -> bool {
    let id = pod.id().to_raw();
    let widget_mut = parent_ctx
        .widget_children
        .get_child_mut(id)
        .expect("WidgetPod: inner widget not found in widget tree");
    let mut state_mut = parent_ctx
        .widget_state_children
        .get_child_mut(id)
        .expect("WidgetPod: inner widget not found in widget tree");
    let widget = widget_mut.item;
    let state = state_mut.item;

    if state.is_stashed {
        debug_panic!(
            "Error in '{}' #{}: trying to compute layout of stashed widget.",
            widget.short_type_name(),
            id,
        );
        state.size = Size::ZERO;
        return false;
    }

    state.needs_compose = true;
    state.is_expecting_place_child_call = true;
    // TODO - Not everything that has been re-laid out needs to be repainted.
    state.needs_paint = true;
    state.request_accessibility = true;
    state.needs_accessibility = true;

    bc.debug_check(widget.short_type_name());
    trace!("Computing layout with constraints {:?}", bc);

    state.local_paint_rect = Rect::ZERO;

    let new_size = {
        let mut inner_ctx = LayoutCtx {
            widget_state: state,
            widget_state_children: state_mut.children.reborrow_mut(),
            widget_children: widget_mut.children,
            global_state: parent_ctx.global_state,
            mouse_pos: parent_ctx.mouse_pos,
        };

        widget.layout(&mut inner_ctx, bc)
    };

    // TODO - One we add request_layout, check for that flag.
    // If it's true, that's probably an error.
    state.needs_layout = false;

    state.local_paint_rect = state
        .local_paint_rect
        .union(new_size.to_rect() + state.paint_insets);

    #[cfg(debug_assertions)]
    {
        for child_id in widget.children_ids() {
            let child_id = child_id.to_raw();
            let child_state_mut = state_mut
                .children
                .get_child_mut(child_id)
                .unwrap_or_else(|| panic!("widget #{child_id} not found"));
            let child_state = child_state_mut.item;
            if child_state.is_expecting_place_child_call {
                debug_panic!(
                        "Error in '{}' #{}: missing call to place_child method for child widget '{}' #{}. During layout pass, if a widget calls WidgetPod::layout() on its child, it then needs to call LayoutCtx::place_child() on the same child.",
                        widget.short_type_name(),
                        id,
                        child_state.widget_name,
                        child_state.id.to_raw(),
                    );
            }

            // TODO - This check might be redundant with the code updating local_paint_rect
            let child_rect = child_state.paint_rect();
            if !rect_contains(&state.local_paint_rect, &child_rect) && !state.is_portal {
                debug_panic!(
                        "Error in '{}' #{}: paint_rect {:?} doesn't contain paint_rect {:?} of child widget '{}' #{}",
                        widget.short_type_name(),
                        id,
                        state.local_paint_rect,
                        child_rect,
                        child_state.widget_name,
                        child_state.id.to_raw(),
                    );
            }
        }
    }

    // TODO - Figure out how to deal with the overflow problem, eg:
    // What happens if a widget returns a size larger than the allowed constraints?
    // Some possibilities are:
    // - Always clip: might be expensive
    // - Display it anyway: might lead to graphical bugs
    // - Panic: too harsh?
    // Also, we need to avoid spurious crashes when we initialize the app and the
    // size is (0,0)
    // See https://github.com/linebender/xilem/issues/377

    let state_mut = parent_ctx
        .widget_state_children
        .get_child_mut(id)
        .expect("WidgetPod: inner widget not found in widget tree");
    parent_ctx.widget_state.merge_up(state_mut.item);
    state_mut.item.size = new_size;

    log_layout_issues(widget.short_type_name(), new_size);

    true
}

fn log_layout_issues(type_name: &str, size: Size) {
    if size.width.is_infinite() {
        warn!("Widget `{type_name}` has an infinite width.");
    }
    if size.height.is_infinite() {
        warn!("Widget `{type_name}` has an infinite height.");
    }
}

pub(crate) fn run_layout_on<W: Widget>(
    parent_ctx: &mut LayoutCtx<'_>,
    pod: &mut WidgetPod<W>,
    bc: &BoxConstraints,
) -> Size {
    pod.call_widget_method_with_checks(
        "layout",
        parent_ctx,
        |ctx| {
            (
                ctx.widget_state_children.reborrow(),
                ctx.widget_children.reborrow(),
            )
        },
        |child, ctx| run_layout_inner(ctx, child, bc),
    );

    let id = pod.id().to_raw();
    let state_mut = parent_ctx
        .widget_state_children
        .get_child_mut(id)
        .expect("run_layout_on: inner widget not found in widget tree");
    state_mut.item.size
}

pub(crate) fn root_layout(
    root: &mut RenderRoot,
    synthetic_root_state: &mut WidgetState,
    bc: &BoxConstraints,
) -> Size {
    let _span = info_span!("layout").entered();

    let mouse_pos = root.last_mouse_pos.map(|pos| (pos.x, pos.y).into());
    let root_state_token = root.widget_arena.widget_states.root_token_mut();
    let root_widget_token = root.widget_arena.widgets.root_token_mut();
    let mut ctx = LayoutCtx {
        global_state: &mut root.state,
        widget_state: synthetic_root_state,
        widget_state_children: root_state_token,
        widget_children: root_widget_token,
        mouse_pos,
    };

    let size = run_layout_on(&mut ctx, &mut root.root, bc);
    ctx.place_child(&mut root.root, Point::ORIGIN);

    size
}
