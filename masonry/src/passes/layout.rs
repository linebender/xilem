// Copyright 2024 the Xilem Authors
// SPDX-License-Identifier: Apache-2.0

//! The layout pass, which provides the size and position of each widget
//! before any translations applied in [`compose`](crate::passes::compose).
//! Most of the logic for this pass happens in [`Widget::layout`] implementations.

use dpi::LogicalSize;
use smallvec::SmallVec;
use tracing::{info_span, trace};
use vello::kurbo::{Point, Rect, Size};

use crate::passes::recurse_on_children;
use crate::render_root::{RenderRoot, RenderRootSignal, WindowSizePolicy};
use crate::widget::{BiAxial, ContentFill, WidgetState};
use crate::{LayoutCtx, Widget, WidgetPod};
use crate::widget::ContentFill::Exact;

// --- MARK: RUN LAYOUT ---
/// Run [`Widget::layout`] method on the widget contained in `pod`.
/// This will be called by [`LayoutCtx::run_layout`], which is itself called in the parent widget's `layout`.
pub(crate) fn run_layout_on<W: Widget>(
    parent_ctx: &mut LayoutCtx<'_>,
    pod: &mut WidgetPod<W>,
    size: &BiAxial<f64>,
    requested_fill: &BiAxial<ContentFill>,
) -> BiAxial<f64> {
    let id = pod.id().to_raw();
    let mut widget = parent_ctx.widget_children.get_child_mut(id).unwrap();
    let mut state = parent_ctx.widget_state_children.get_child_mut(id).unwrap();

    let _span = widget.item.make_trace_span().entered();

    let mut children_ids = SmallVec::new();
    if cfg!(debug_assertions) {
        children_ids = widget.item.children_ids();

        // We forcefully set request_layout to true for all children.
        // This is used below to check that widget.layout(..) visited all of them.
        for child_id in widget.item.children_ids() {
            let child_id = child_id.to_raw();
            let child_state = state.children.get_child_mut(child_id).unwrap().item;
            if !child_state.is_stashed {
                child_state.request_layout = true;
            }
        }
    }

    // This checks reads is_explicitly_stashed instead of is_stashed because the latter may be outdated.
    // A widget's is_explicitly_stashed flag is controlled by its direct parent.
    // The parent may set this flag during layout, in which case it should avoid calling run_layout.
    // Note that, because this check exits before recursing, run_layout can only ever be
    // reached for a widget whose parent is not stashed, which means is_explicitly_stashed
    // being false is sufficient to know the widget is non-stashed.
    if state.item.is_explicitly_stashed {
        debug_panic!(
            "Error in '{}' {}: trying to compute layout of stashed widget.",
            widget.item.short_type_name(),
            pod.id(),
        );
        state.item.size = BiAxial::ZERO;
        return BiAxial::ZERO;
    }

    // TODO - Not everything that has been re-laid out needs to be repainted.
    state.item.needs_paint = true;
    state.item.needs_compose = true;
    state.item.needs_accessibility = true;
    state.item.request_paint = true;
    state.item.request_compose = true;
    state.item.request_accessibility = true;

    size.validate_sizes(widget.item.short_type_name());
    trace!("Computing layout with constraints {:?}", size);

    state.item.local_paint_rect = Rect::ZERO;

    // TODO - Handle more elegantly
    // We suppress need_layout and request_layout for stashed children
    // to avoid unnecessary relayouts in corner cases.
    recurse_on_children(
        pod.id(),
        widget.reborrow_mut(),
        state.children.reborrow_mut(),
        |_, state| {
            if state.item.is_stashed {
                state.item.needs_layout = false;
                state.item.request_layout = false;
            }
        },
    );

    let new_size = {
        let mut inner_ctx = LayoutCtx {
            widget_state: state.item,
            widget_state_children: state.children.reborrow_mut(),
            widget_children: widget.children,
            global_state: parent_ctx.global_state,
        };

        // TODO - If constraints are the same and request_layout isn't set,
        // skip calling layout
        inner_ctx.widget_state.request_layout = false;
        widget.item.layout(&mut inner_ctx, size, requested_fill)
    };
    if state.item.request_layout {
        debug_panic!(
            "Error in '{}' {}: layout request flag was set during layout pass",
            widget.item.short_type_name(),
            pod.id(),
        );
    }
    trace!(
        "Computed layout: size={}, baseline={}, insets={:?}",
        new_size,
        state.item.baseline_offset,
        state.item.paint_insets,
    );

    state.item.needs_layout = false;
    state.item.is_expecting_place_child_call = true;

    state.item.local_paint_rect = state
        .item
        .local_paint_rect
        .union(new_size.to_size().to_rect() + state.item.paint_insets);

    #[cfg(debug_assertions)]
    {
        let name = widget.item.short_type_name();
        for child_id in widget.item.children_ids() {
            let child_id = child_id.to_raw();
            let child_state = state.children.get_child_mut(child_id).unwrap().item;

            if child_state.is_stashed {
                continue;
            }

            if child_state.request_layout {
                debug_panic!(
                    "Error in '{}' {}: LayoutCtx::run_layout() was not called with child widget '{}' {}.",
                    name,
                    pod.id(),
                    child_state.widget_name,
                    child_state.id,
                );
            }

            if child_state.is_expecting_place_child_call {
                debug_panic!(
                    "Error in '{}' {}: LayoutCtx::place_child() was not called with child widget '{}' {}.",
                    name,
                    pod.id(),
                    child_state.widget_name,
                    child_state.id,
                );
            }

            // TODO - This check might be redundant with the code updating local_paint_rect
            let child_rect = child_state.paint_rect();
            if !state.item.local_paint_rect.contains_rect(child_rect) && state.item.clip.is_none() {
                debug_panic!(
                    "Error in '{}' {}: paint_rect {:?} doesn't contain paint_rect {:?} of child widget '{}' {}",
                    name,
                    pod.id(),
                    state.item.local_paint_rect,
                    child_rect,
                    child_state.widget_name,
                    child_state.id,
                );
            }
        }

        let new_children_ids = widget.item.children_ids();
        if children_ids != new_children_ids && !state.item.children_changed {
            debug_panic!(
                "Error in '{}' {}: children changed during layout pass",
                name,
                pod.id(),
            );
        }

        if !new_size.horizontal.is_finite() || !new_size.vertical.is_finite() {
            debug_panic!(
                "Error in '{}' {}: invalid size {}",
                name,
                pod.id(),
                new_size
            );
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

    let state_mut = parent_ctx.widget_state_children.get_child_mut(id).unwrap();
    parent_ctx.widget_state.merge_up(state_mut.item);
    state_mut.item.size = new_size;
    new_size
}

// --- MARK: ROOT ---
pub(crate) fn run_layout_pass(root: &mut RenderRoot) {
    if !root.root_state().needs_layout {
        return;
    }

    let _span = info_span!("layout").entered();

    let window_size = root.get_planar_size();
    let input_size = match root.size_policy {
        WindowSizePolicy::User => window_size, // Note: this was "tight"
        WindowSizePolicy::Content => BiAxial::UNBOUNDED,
    };
    let content_fill = BiAxial {
        horizontal: Exact,
        vertical: Exact,
    };

    let mut dummy_state = WidgetState::synthetic(root.root.id(), root.get_planar_size());
    let root_state_token = root.widget_arena.widget_states.root_token_mut();
    let root_widget_token = root.widget_arena.widgets.root_token_mut();
    let mut ctx = LayoutCtx {
        global_state: &mut root.state,
        widget_state: &mut dummy_state,
        widget_state_children: root_state_token,
        widget_children: root_widget_token,
    };

    let size = run_layout_on(&mut ctx, &mut root.root, &input_size, &content_fill);
    ctx.place_child(&mut root.root, Point::ORIGIN);

    if let WindowSizePolicy::Content = root.size_policy {
        let new_size = LogicalSize::new(size.horizontal, size.vertical).to_physical(root.scale_factor);
        if root.size != new_size {
            root.size = new_size;
            root.state.emit_signal(RenderRootSignal::SetSize(new_size));
        }
    }
}
