// Copyright 2024 the Xilem Authors
// SPDX-License-Identifier: Apache-2.0

//! The layout pass, which provides the size and position of each widget
//! before any translations applied in [`compose`](crate::passes::compose).
//! Most of the logic for this pass happens in [`Widget::layout`] implementations.

use dpi::LogicalSize;
use smallvec::SmallVec;
use tracing::{info_span, trace};
use vello::kurbo::{Point, Rect, Size};

use crate::app::{RenderRoot, RenderRootSignal, WindowSizePolicy};
use crate::core::{BoxConstraints, LayoutCtx, PropertiesMut, Widget, WidgetPod, WidgetState};
use crate::debug_panic;
use crate::passes::{enter_span_if, recurse_on_children2};

// --- MARK: RUN LAYOUT
/// Run [`Widget::layout`] method on the widget contained in `pod`.
/// This will be called by [`LayoutCtx::run_layout`], which is itself called in the parent widget's `layout`.
pub(crate) fn run_layout_on<W: Widget + ?Sized>(
    parent_ctx: &mut LayoutCtx<'_>,
    pod: &mut WidgetPod<W>,
    bc: &BoxConstraints,
) -> Size {
    let id = pod.id();
    let (item, mut children) = parent_ctx.children.child_mut(id).unwrap();

    let trace = parent_ctx.global_state.trace.layout;
    let _span = enter_span_if(trace, &**item.widget, item.state.id);

    let mut children_ids = SmallVec::new();
    if cfg!(debug_assertions) {
        children_ids = item.widget.children_ids();

        // We forcefully set request_layout to true for all children.
        // This is used below to check that widget.layout(..) visited all of them.
        for child_id in item.widget.children_ids() {
            let child_state = children.state_children.item_mut(child_id).unwrap().item;
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
    if item.state.is_explicitly_stashed {
        debug_panic!(
            "Error in '{}' {}: trying to compute layout of stashed widget.",
            item.widget.short_type_name(),
            pod.id(),
        );
        item.state.size = Size::ZERO;
        return Size::ZERO;
    }

    // TODO - Not everything that has been re-laid out needs to be repainted.
    item.state.needs_paint = true;
    item.state.needs_compose = true;
    item.state.needs_accessibility = true;
    item.state.request_paint = true;
    item.state.request_compose = true;
    item.state.request_accessibility = true;

    bc.debug_check(item.widget.short_type_name());
    if trace {
        trace!("Computing layout with constraints {:?}", bc);
    }

    item.state.local_paint_rect = Rect::ZERO;

    // TODO - Handle more elegantly
    // We suppress need_layout and request_layout for stashed children
    // to avoid unnecessary relayouts in corner cases.
    recurse_on_children2(
        pod.id(),
        &**item.widget,
        children.reborrow_mut(),
        |item, _| {
            if item.state.is_stashed {
                item.state.needs_layout = false;
                item.state.request_layout = false;
            }
        },
    );

    let new_size = {
        let mut inner_ctx = LayoutCtx {
            widget_state: item.state,
            children: children.reborrow_mut(),
            default_properties: parent_ctx.default_properties,
            global_state: parent_ctx.global_state,
        };

        // TODO - If constraints are the same and request_layout isn't set,
        // skip calling layout
        inner_ctx.widget_state.request_layout = false;
        let mut props = PropertiesMut {
            map: item.properties,
            default_map: parent_ctx
                .default_properties
                .for_widget(item.widget.type_id()),
        };
        item.widget.layout(&mut inner_ctx, &mut props, bc)
    };
    if item.state.request_layout {
        debug_panic!(
            "Error in '{}' {}: layout request flag was set during layout pass",
            item.widget.short_type_name(),
            pod.id(),
        );
    }
    if trace {
        trace!(
            "Computed layout: size={}, baseline={}, insets={:?}",
            new_size, item.state.baseline_offset, item.state.paint_insets,
        );
    }

    item.state.needs_layout = false;
    item.state.is_expecting_place_child_call = true;

    item.state.local_paint_rect = item
        .state
        .local_paint_rect
        .union(new_size.to_rect() + item.state.paint_insets);

    #[cfg(debug_assertions)]
    {
        let name = item.widget.short_type_name();
        for child_id in item.widget.children_ids() {
            let child_state = children.state_children.item_mut(child_id).unwrap().item;

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
        }

        let new_children_ids = item.widget.children_ids();
        if children_ids != new_children_ids && !item.state.children_changed {
            debug_panic!(
                "Error in '{}' {}: children changed during layout pass",
                name,
                pod.id(),
            );
        }

        if !new_size.width.is_finite() || !new_size.height.is_finite() {
            debug_panic!(
                "Error in '{}' {}: invalid size {}",
                name,
                pod.id(),
                new_size
            );
        }
    }

    let state_mut = parent_ctx.children.state_children.item_mut(id).unwrap();
    parent_ctx.widget_state.merge_up(state_mut.item);
    state_mut.item.size = new_size;
    new_size
}

// --- MARK: ROOT
/// See the [passes documentation](../doc/05_pass_system.md#layout-pass).
pub(crate) fn run_layout_pass(root: &mut RenderRoot) {
    if !root.root_state().needs_layout {
        return;
    }

    let _span = info_span!("layout").entered();
    root.global_state.needs_pointer_pass = true;

    let window_size = root.get_kurbo_size();
    let bc = match root.size_policy {
        WindowSizePolicy::User => BoxConstraints::tight(window_size),
        WindowSizePolicy::Content => BoxConstraints::UNBOUNDED,
    };

    let mut dummy_state = WidgetState::synthetic(root.root.id(), root.get_kurbo_size());
    let mut ctx = LayoutCtx {
        global_state: &mut root.global_state,
        widget_state: &mut dummy_state,
        children: root.widget_arena.roots_mut(),
        default_properties: &root.default_properties,
    };

    let size = run_layout_on(&mut ctx, &mut root.root, &bc);
    ctx.place_child(&mut root.root, Point::ORIGIN);

    if let WindowSizePolicy::Content = root.size_policy {
        let new_size =
            LogicalSize::new(size.width, size.height).to_physical(root.global_state.scale_factor);
        if root.size != new_size {
            root.size = new_size;
            root.global_state
                .emit_signal(RenderRootSignal::SetSize(new_size));
        }
    }
}
