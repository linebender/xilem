// Copyright 2024 the Xilem Authors
// SPDX-License-Identifier: Apache-2.0

//! The layout pass, which provides the size and position of each widget
//! before any translations applied in [`compose`](crate::passes::compose).
//! Most of the logic for this pass happens in [`Widget::layout`] implementations.

use smallvec::SmallVec;
use tracing::{info_span, trace};
use vello::kurbo::{Point, Rect, Size};

use crate::render_root::RenderRoot;
use crate::tree_arena::ArenaRefChildren;
use crate::widget::WidgetState;
use crate::{BoxConstraints, LayoutCtx, Widget, WidgetPod};

// TODO - Replace with contains_rect once new Kurbo version is released.
// See https://github.com/linebender/kurbo/pull/347
/// Return `true` if all of `smaller` is within `larger`.
fn rect_contains(larger: &Rect, smaller: &Rect) -> bool {
    smaller.x0 >= larger.x0
        && smaller.x1 <= larger.x1
        && smaller.y0 >= larger.y0
        && smaller.y1 <= larger.y1
}

// TODO - document
// TODO - This method should take a 'can_skip: Fn(WidgetRef) -> bool'
// predicate and only panic if can_skip returns false.
// TODO - This method was copy-pasted from WidgetPod. It was originally used
// in multiple passes, but it's only used in layout right now. It should be
// rewritten with that in mind.
#[inline(always)]
fn call_widget_method_with_checks<W: Widget, Ctx>(
    pod: &mut WidgetPod<W>,
    method_name: &str,
    ctx: &mut Ctx,
    get_tokens: impl Fn(
        &mut Ctx,
    ) -> (
        ArenaRefChildren<'_, WidgetState>,
        ArenaRefChildren<'_, Box<dyn Widget>>,
    ),
    visit: impl FnOnce(&mut WidgetPod<W>, &mut Ctx) -> bool,
) {
    if pod.incomplete() {
        debug_panic!(
            "Error in widget #{}: method '{}' called before receiving WidgetAdded.",
            pod.id().to_raw(),
            method_name,
        );
    }

    let id = pod.id().to_raw();
    let (parent_state_mut, parent_token) = get_tokens(ctx);
    let widget_ref = parent_token
        .get_child(id)
        .expect("WidgetPod: inner widget not found in widget tree");
    let state_ref = parent_state_mut
        .get_child(id)
        .expect("WidgetPod: inner widget not found in widget tree");
    let widget = widget_ref.item;
    let state = state_ref.item;

    let _span = widget.make_trace_span().entered();

    // TODO https://github.com/linebender/xilem/issues/370 - Re-implement debug logger

    // TODO - explain this
    state.mark_as_visited(true);

    let mut children_ids = SmallVec::new();

    if cfg!(debug_assertions) {
        for child_state_ref in state_ref.children.iter_children() {
            child_state_ref.item.mark_as_visited(false);
        }
        children_ids = widget.children_ids();
    }

    let called_widget = visit(pod, ctx);

    let (parent_state_mut, parent_token) = get_tokens(ctx);
    let widget_ref = parent_token
        .get_child(id)
        .expect("WidgetPod: inner widget not found in widget tree");
    let state_ref = parent_state_mut
        .get_child(id)
        .expect("WidgetPod: inner widget not found in widget tree");
    let widget = widget_ref.item;
    let state = state_ref.item;

    if cfg!(debug_assertions) && called_widget {
        let new_children_ids = widget.children_ids();
        if children_ids != new_children_ids && !state.children_changed {
            debug_panic!(
                    "Error in '{}' #{}: children changed in method {} but ctx.children_changed() wasn't called",
                    widget.short_type_name(),
                    pod.id().to_raw(),
                    method_name,
                );
        }

        for id in &new_children_ids {
            let id = id.to_raw();
            if !state_ref.children.has_child(id) {
                debug_panic!(
                    "Error in '{}' #{}: child widget #{} not added in method {}",
                    widget.short_type_name(),
                    pod.id().to_raw(),
                    id,
                    method_name,
                );
            }
        }

        #[cfg(debug_assertions)]
        for child_state_ref in state_ref.children.iter_children() {
            // FIXME - use can_skip callback instead
            if child_state_ref.item.needs_visit() && !child_state_ref.item.is_stashed {
                debug_panic!(
                    "Error in '{}' #{}: child widget '{}' #{} not visited in method {}",
                    widget.short_type_name(),
                    pod.id().to_raw(),
                    child_state_ref.item.widget_name,
                    child_state_ref.item.id.to_raw(),
                    method_name,
                );
            }
        }
    }
}

// Returns "true" if the Widget's layout method was called, in which case debug checks
// need to be run. (See 'called_widget' in WidgetPod::call_widget_method_with_checks)
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

    // The parent (and only the parent) controls the stashed state, and it is valid to set `stashed` in layout.
    // Because of that, we use the local value rather than the global value.
    // Note that if we are stashed by a grandparent, this check would trigger for that grandparent, so we should
    // never be called.
    if state.is_explicitly_stashed {
        debug_panic!(
            "Error in '{}' #{}: trying to compute layout of stashed widget.",
            widget.short_type_name(),
            id,
        );
        state.size = Size::ZERO;
        return false;
    }

    // TODO - Not everything that has been re-laid out needs to be repainted.
    state.needs_paint = true;
    state.needs_compose = true;
    state.needs_accessibility = true;
    state.request_paint = true;
    state.request_compose = true;
    state.request_accessibility = true;

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

        // TODO - If constraints are the same and request_layout isn't set,
        // skip calling layout
        inner_ctx.widget_state.request_layout = false;
        widget.layout(&mut inner_ctx, bc)
    };
    if state.request_layout {
        debug_panic!(
            "Error in '{}' #{}: layout request flag was set during layout pass",
            widget.short_type_name(),
            id,
        );
    }

    state.needs_layout = false;
    state.is_expecting_place_child_call = true;

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

    {
        if new_size.width.is_infinite() {
            debug_panic!(
                "Error in '{}' #{}: width is infinite",
                widget.short_type_name(),
                id,
            );
        }
        if new_size.height.is_infinite() {
            debug_panic!(
                "Error in '{}' #{}: height is infinite",
                widget.short_type_name(),
                id,
            );
        }
    }

    true
}

/// Run [`Widget::layout`] method on the widget contained in `pod`.
/// This will be called by [`LayoutCtx::run_layout`], which is itself called in the parent widget's `layout`.
pub(crate) fn run_layout_on<W: Widget>(
    parent_ctx: &mut LayoutCtx<'_>,
    pod: &mut WidgetPod<W>,
    bc: &BoxConstraints,
) -> Size {
    call_widget_method_with_checks(
        pod,
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
