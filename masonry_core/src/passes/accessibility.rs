// Copyright 2024 the Xilem Authors
// SPDX-License-Identifier: Apache-2.0

use accesskit::{Node, NodeId, Tree, TreeUpdate};
use anymap3::AnyMap;
use tracing::{debug, info_span, trace};
use tree_arena::ArenaMut;
use vello::kurbo::Rect;

use crate::app::{RenderRoot, RenderRootState};
use crate::core::{AccessCtx, PropertiesRef, Widget, WidgetState};
use crate::passes::{enter_span_if, recurse_on_children};

// --- MARK: BUILD TREE ---
fn build_accessibility_tree(
    global_state: &mut RenderRootState,
    tree_update: &mut TreeUpdate,
    mut widget: ArenaMut<'_, Box<dyn Widget>>,
    mut state: ArenaMut<'_, WidgetState>,
    mut properties: ArenaMut<'_, AnyMap>,
    rebuild_all: bool,
    scale_factor: Option<f64>,
) {
    let _span = enter_span_if(
        global_state.trace.access,
        global_state,
        widget.reborrow(),
        state.reborrow(),
        properties.reborrow(),
    );
    let id = state.item.id;

    if !rebuild_all && !state.item.needs_accessibility {
        return;
    }

    if rebuild_all || state.item.request_accessibility {
        if global_state.trace.access {
            trace!(
                "Building accessibility node for widget '{}' {}",
                widget.item.short_type_name(),
                id,
            );
        }

        let mut ctx = AccessCtx {
            global_state,
            widget_state: state.item,
            widget_state_children: state.children.reborrow_mut(),
            widget_children: widget.children.reborrow_mut(),
            properties_children: properties.children.reborrow_mut(),
            tree_update,
            rebuild_all,
        };
        let mut node = build_access_node(&mut **widget.item, &mut ctx, scale_factor);
        let props = PropertiesRef {
            map: properties.item,
        };
        widget.item.accessibility(&mut ctx, &props, &mut node);

        let id: NodeId = ctx.widget_state.id.into();
        if ctx.global_state.trace.access {
            trace!("Built node {} with role={:?}", id.0, node.role());
        }
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
        properties.children,
        |widget, mut state, properties| {
            // TODO - We don't skip updating stashed items because doing so
            // is error-prone. We may want to revisit that decision.
            build_accessibility_tree(
                global_state,
                tree_update,
                widget,
                state.reborrow_mut(),
                properties,
                rebuild_all,
                None,
            );
            parent_state.merge_up(state.item);
        },
    );
}

// --- MARK: BUILD NODE ---
fn build_access_node(
    widget: &mut dyn Widget,
    ctx: &mut AccessCtx,
    scale_factor: Option<f64>,
) -> Node {
    let mut node = Node::new(widget.accessibility_role());
    node.set_bounds(to_accesskit_rect(ctx.widget_state.size.to_rect()));

    let local_translation = ctx.widget_state.scroll_translation + ctx.widget_state.origin.to_vec2();
    let mut local_transform = ctx.widget_state.transform.then_translate(local_translation);

    if let Some(scale_factor) = scale_factor {
        local_transform = local_transform.pre_scale(scale_factor);
    }
    node.set_transform(accesskit::Affine::new(local_transform.as_coeffs()));

    node.set_children(
        widget
            .children_ids()
            .iter()
            .copied()
            .map(|id| id.into())
            .collect::<Vec<NodeId>>(),
    );

    // Note - The values returned by these methods can be modified by other passes.
    // When that happens, the other pass should set flags to request an accessibility pass.
    if ctx.is_disabled() {
        node.set_disabled();
    }
    if ctx.is_stashed() {
        node.set_hidden();
    }
    if ctx.widget_state.clip_path.is_some() {
        node.set_clips_children();
    }
    if ctx.accepts_focus() && !ctx.is_disabled() && !ctx.is_stashed() {
        node.add_action(accesskit::Action::Focus);
    }
    if ctx.is_focus_target() {
        node.add_action(accesskit::Action::Blur);
    }

    node
}

fn to_accesskit_rect(r: Rect) -> accesskit::Rect {
    accesskit::Rect::new(r.x0, r.y0, r.x1, r.y1)
}

// --- MARK: ROOT ---
/// See the [passes documentation](../doc/05_pass_system.md#render-passes).
pub(crate) fn run_accessibility_pass(root: &mut RenderRoot, scale_factor: f64) -> TreeUpdate {
    let _span = info_span!("accessibility").entered();

    let mut tree_update = TreeUpdate {
        nodes: vec![],
        tree: Some(Tree {
            root: root.root.id().into(),
            toolkit_name: Some("Masonry".to_string()),
            toolkit_version: Some(env!("CARGO_PKG_VERSION").to_string()),
        }),
        focus: root
            .global_state
            .focused_widget
            .unwrap_or(root.root.id())
            .into(),
    };

    let (root_widget, root_state, root_properties) = {
        let widget_id = root.root.id();
        let widget = root
            .widget_arena
            .widgets
            .find_mut(widget_id)
            .expect("root_accessibility: root not in widget tree");
        let state = root
            .widget_arena
            .states
            .find_mut(widget_id)
            .expect("root_accessibility: root state not in widget tree");
        let properties = root
            .widget_arena
            .properties
            .find_mut(widget_id)
            .expect("root_accessibility: root properties not in widget tree");
        (widget, state, properties)
    };

    if root.rebuild_access_tree {
        debug!("Running ACCESSIBILITY pass with rebuild_all");
    }
    build_accessibility_tree(
        &mut root.global_state,
        &mut tree_update,
        root_widget,
        root_state,
        root_properties,
        root.rebuild_access_tree,
        Some(scale_factor),
    );
    root.rebuild_access_tree = false;

    tree_update
}
