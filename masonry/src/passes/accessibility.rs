// Copyright 2024 the Xilem Authors
// SPDX-License-Identifier: Apache-2.0

use crate::render_root::RenderRoot;
use crate::tree_arena::ArenaMutChildren;
use accesskit::{NodeBuilder, NodeId, TreeUpdate};
use tracing::debug;
use tracing::info_span;
use tracing::trace;
use vello::kurbo::Rect;

use crate::passes::recurse_on_children;
use crate::render_root::RenderRootState;
use crate::tree_arena::ArenaMut;
use crate::{AccessCtx, Widget, WidgetState};

fn build_accessibility_tree(
    global_state: &mut RenderRootState,
    tree_update: &mut TreeUpdate,
    mut widget: ArenaMut<'_, Box<dyn Widget>>,
    mut state: ArenaMut<'_, WidgetState>,
    rebuild_all: bool,
    scale_factor: f64,
) {
    let _span = widget.item.make_trace_span().entered();
    let id = state.item.id;

    if !rebuild_all && !state.item.needs_accessibility {
        return;
    }

    if rebuild_all || state.item.request_accessibility {
        trace!(
            "Building accessibility node for widget '{}' #{}",
            widget.item.short_type_name(),
            id.to_raw(),
        );
        let current_node = build_access_node(
            widget.item,
            state.item,
            state.children.reborrow_mut(),
            scale_factor,
        );

        let mut ctx = AccessCtx {
            global_state,
            widget_state: state.item,
            widget_state_children: state.children.reborrow_mut(),
            widget_children: widget.children.reborrow_mut(),
            tree_update,
            current_node,
            rebuild_all,
            scale_factor,
        };
        widget.item.accessibility(&mut ctx);

        let id: NodeId = ctx.widget_state.id.into();
        trace!(
            "Built node #{} with role={:?}, default_action={:?}",
            id.0,
            ctx.current_node.role(),
            ctx.current_node.default_action_verb(),
        );
        ctx.tree_update.nodes.push((id, ctx.current_node.build()));
    }

    state.item.request_accessibility = false;
    state.item.needs_accessibility = false;

    let id = state.item.id;
    let parent_state = state.item;
    recurse_on_children(
        id,
        widget.reborrow_mut(),
        state.children,
        |widget, mut state| {
            // TODO - We skip updating stashed items.
            // This may have knock-on effects we'd need to document.
            if state.item.is_stashed {
                return;
            }
            build_accessibility_tree(
                global_state,
                tree_update,
                widget,
                state.reborrow_mut(),
                rebuild_all,
                scale_factor,
            );
            parent_state.merge_up(state.item);
        },
    );
}

fn build_access_node(
    widget: &dyn Widget,
    state: &WidgetState,
    state_children: ArenaMutChildren<'_, WidgetState>,
    scale_factor: f64,
) -> NodeBuilder {
    let mut node = NodeBuilder::new(widget.accessibility_role());
    node.set_bounds(to_accesskit_rect(state.window_layout_rect(), scale_factor));

    // TODO - We skip listing stashed items.
    // This may have knock-on effects we'd need to document.
    node.set_children(
        widget
            .children_ids()
            .iter()
            .copied()
            .filter(|id| {
                !state_children
                    .get_child(id.to_raw())
                    .unwrap()
                    .item
                    .is_stashed
            })
            .map(|id| id.into())
            .collect::<Vec<NodeId>>(),
    );

    if state.is_hot {
        node.set_hovered();
    }
    if state.is_disabled() {
        node.set_disabled();
    }
    if state.is_stashed {
        node.set_hidden();
    }
    if state.clip.is_some() {
        node.set_clips_children();
    }

    node
}

fn to_accesskit_rect(r: Rect, scale_factor: f64) -> accesskit::Rect {
    let s = scale_factor;
    accesskit::Rect::new(s * r.x0, s * r.y0, s * r.x1, s * r.y1)
}

// ----------------

pub(crate) fn root_accessibility(
    root: &mut RenderRoot,
    rebuild_all: bool,
    scale_factor: f64,
) -> TreeUpdate {
    let _span = info_span!("accessibility").entered();

    let mut tree_update = TreeUpdate {
        nodes: vec![],
        tree: None,
        focus: root.state.focused_widget.unwrap_or(root.root.id()).into(),
    };

    let (root_widget, root_state) = {
        let widget_id = root.root.id();
        let widget = root
            .widget_arena
            .widgets
            .find_mut(widget_id.to_raw())
            .expect("root_accessibility: root not in widget tree");
        let state = root
            .widget_arena
            .widget_states
            .find_mut(widget_id.to_raw())
            .expect("root_accessibility: root state not in widget tree");
        (widget, state)
    };

    if rebuild_all {
        debug!("Running ACCESSIBILITY pass with rebuild_all");
    }

    build_accessibility_tree(
        &mut root.state,
        &mut tree_update,
        root_widget,
        root_state,
        rebuild_all,
        scale_factor,
    );

    tree_update
}
