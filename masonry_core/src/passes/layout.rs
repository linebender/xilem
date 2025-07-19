// Copyright 2024 the Xilem Authors
// SPDX-License-Identifier: Apache-2.0

//! The layout pass, which provides the size and position of each widget
//! before any translations applied in [`compose`](crate::passes::compose).
//! Most of the logic for this pass happens in [`Widget::layout`] implementations.

use dpi::LogicalSize;
use smallvec::SmallVec;
use tracing::{info_span, trace};
use tree_arena::ArenaMut;
use vello::kurbo::{Rect, Size};

use crate::app::{RenderRoot, RenderRootSignal, RenderRootState, WindowSizePolicy};
use crate::core::{
    BoxConstraints, DefaultProperties, LayoutCtx, PropertiesMut, Widget, WidgetState,
};
use crate::debug_panic;
use crate::passes::{enter_span_if, recurse_on_children};
use crate::util::AnyMap;

// --- MARK: RUN LAYOUT
/// Run [`Widget::layout`] method on the given widget.
/// This will be called by [`LayoutCtx::run_layout`], which is itself called in the parent widget's `layout`.
pub(crate) fn run_layout_on(
    global_state: &mut RenderRootState,
    default_properties: &DefaultProperties,
    mut widget: ArenaMut<'_, Box<dyn Widget>>,
    mut state: ArenaMut<'_, WidgetState>,
    mut properties: ArenaMut<'_, AnyMap>,
    bc: &BoxConstraints,
) -> Size {
    let id = state.item.id;
    let trace = global_state.trace.layout;
    let _span = enter_span_if(trace, state.reborrow());

    let mut children_ids = SmallVec::new();
    if cfg!(debug_assertions) {
        children_ids = widget.item.children_ids();

        // We forcefully set request_layout to true for all children.
        // This is used below to check that widget.layout(..) visited all of them.
        for child_id in widget.item.children_ids() {
            let child_state = state.children.item_mut(child_id).unwrap().item;
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
            id,
        );
        state.item.size = Size::ZERO;
        return Size::ZERO;
    }

    // TODO - Not everything that has been re-laid out needs to be repainted.
    state.item.needs_paint = true;
    state.item.needs_compose = true;
    state.item.needs_accessibility = true;
    state.item.request_paint = true;
    state.item.request_compose = true;
    state.item.request_accessibility = true;

    bc.debug_check(widget.item.short_type_name());
    if trace {
        trace!("Computing layout with constraints {:?}", bc);
    }

    state.item.local_paint_rect = Rect::ZERO;

    // TODO - Handle more elegantly
    // We suppress need_layout and request_layout for stashed children
    // to avoid unnecessary relayouts in corner cases.
    recurse_on_children(
        id,
        widget.reborrow_mut(),
        state.children.reborrow_mut(),
        properties.children.reborrow_mut(),
        |_, state, _| {
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
            properties_children: properties.children.reborrow_mut(),
            default_properties,
            global_state,
        };

        // TODO - If constraints are the same and request_layout isn't set,
        // skip calling layout
        inner_ctx.widget_state.request_layout = false;
        let mut props = PropertiesMut {
            map: properties.item,
            default_map: default_properties.for_widget(widget.item.type_id()),
        };
        widget.item.layout(&mut inner_ctx, &mut props, bc)
    };
    if state.item.request_layout {
        debug_panic!(
            "Error in '{}' {}: layout request flag was set during layout pass",
            widget.item.short_type_name(),
            id,
        );
    }
    if trace {
        trace!(
            "Computed layout: size={}, baseline={}, insets={:?}",
            new_size, state.item.baseline_offset, state.item.paint_insets,
        );
    }

    state.item.needs_layout = false;
    state.item.is_expecting_place_child_call = true;

    state.item.local_paint_rect = state
        .item
        .local_paint_rect
        .union(new_size.to_rect() + state.item.paint_insets);

    #[cfg(debug_assertions)]
    {
        let name = widget.item.short_type_name();
        for child_id in widget.item.children_ids() {
            let child_state = state.children.item_mut(child_id).unwrap().item;

            if child_state.is_stashed {
                continue;
            }

            if child_state.request_layout {
                debug_panic!(
                    "Error in '{}' {}: LayoutCtx::run_layout() was not called with child widget '{}' {}.",
                    name,
                    id,
                    child_state.widget_name,
                    child_state.id,
                );
            }

            if child_state.is_expecting_place_child_call {
                debug_panic!(
                    "Error in '{}' {}: LayoutCtx::place_child() was not called with child widget '{}' {}.",
                    name,
                    id,
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
                id,
            );
        }

        if !new_size.width.is_finite() || !new_size.height.is_finite() {
            debug_panic!("Error in '{}' {}: invalid size {}", name, id, new_size);
        }
    }

    state.item.size = new_size;
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

    let (root_widget, mut root_state, root_properties) = {
        let widget_id = root.root.id();
        let widget = root
            .widget_arena
            .widgets
            .find_mut(widget_id)
            .expect("run_layout_pass: root not in widget tree");
        let state = root
            .widget_arena
            .states
            .find_mut(widget_id)
            .expect("run_layout_pass: root state not in widget tree");
        let properties = root
            .widget_arena
            .properties
            .find_mut(widget_id)
            .expect("run_layout_pass: root properties not in widget tree");
        (widget, state, properties)
    };

    let size = run_layout_on(
        &mut root.global_state,
        &root.default_properties,
        root_widget,
        root_state.reborrow_mut(),
        root_properties,
        &bc,
    );
    root_state.item.is_expecting_place_child_call = false;

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
