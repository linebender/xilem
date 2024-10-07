// Copyright 2024 the Xilem Authors
// SPDX-License-Identifier: Apache-2.0

use crate::render_root::RenderRoot;
use accesskit::{Node, NodeBuilder, NodeId, TreeUpdate};
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
            "Building accessibility node for widget '{}' {}",
            widget.item.short_type_name(),
            id,
        );

        let mut ctx = AccessCtx {
            global_state,
            widget_state: state.item,
            widget_state_children: state.children.reborrow_mut(),
            widget_children: widget.children.reborrow_mut(),
            tree_update,
            rebuild_all,
            scale_factor,
        };
        let node = build_access_node(widget.item, &mut ctx);

        let id: NodeId = ctx.widget_state.id.into();
        trace!(
            "Built node {} with role={:?}, default_action={:?}",
            id.0,
            node.role(),
            node.default_action_verb(),
        );
        ctx.tree_update.nodes.push((id, node));
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
            // TODO - We don't skip updating stashed items because doing so
            // is error-prone. We may want to revisit that decision.
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

fn build_access_node(widget: &mut dyn Widget, ctx: &mut AccessCtx) -> Node {
    let mut node = NodeBuilder::new(widget.accessibility_role());
    node.set_bounds(to_accesskit_rect(
        ctx.widget_state.window_layout_rect(),
        ctx.scale_factor,
    ));

    node.set_children(
        widget
            .children_ids()
            .iter()
            .copied()
            .map(|id| id.into())
            .collect::<Vec<NodeId>>(),
    );

    if ctx.is_hot() {
        node.set_hovered();
    }
    if ctx.is_disabled() {
        node.set_disabled();
    }
    if ctx.is_stashed() {
        node.set_hidden();
    }
    if ctx.widget_state.clip.is_some() {
        node.set_clips_children();
    }
    if ctx.is_in_focus_chain() && !ctx.is_disabled() {
        node.add_action(accesskit::Action::Focus);
    }
    if ctx.is_focused() {
        node.add_action(accesskit::Action::Blur);
    }

    widget.accessibility(ctx, &mut node);

    node.build()
}

fn to_accesskit_rect(r: Rect, scale_factor: f64) -> accesskit::Rect {
    let sr = r.scale_from_origin(scale_factor);
    accesskit::Rect::new(sr.x0, sr.y0, sr.x1, sr.y1)
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
