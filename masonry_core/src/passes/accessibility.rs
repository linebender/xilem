// Copyright 2024 the Xilem Authors
// SPDX-License-Identifier: Apache-2.0

use accesskit::{Node, NodeId, Role, Tree, TreeUpdate};
use tracing::{debug, info_span, trace};
use vello::kurbo::Rect;

use crate::app::{RenderRoot, RenderRootState};
use crate::core::{
    AccessCtx, DefaultProperties, PropertiesRef, Widget, WidgetArenaMut, WidgetItemMut,
};
use crate::passes::enter_span_if;

// --- MARK: BUILD TREE
fn build_accessibility_tree(
    global_state: &mut RenderRootState,
    default_properties: &DefaultProperties,
    tree_update: &mut TreeUpdate,
    item: WidgetItemMut<'_>,
    mut children: WidgetArenaMut<'_>,
    rebuild_all: bool,
    scale_factor: Option<f64>,
) {
    let _span = enter_span_if(global_state.trace.access, &**item.widget, item.state.id);
    let id = item.state.id;

    if !rebuild_all && !item.state.needs_accessibility {
        return;
    }

    if rebuild_all || item.state.request_accessibility {
        if global_state.trace.access {
            trace!(
                "Building accessibility node for widget '{}' {}",
                item.widget.short_type_name(),
                id,
            );
        }

        let mut ctx = AccessCtx {
            global_state,
            widget_state: item.state,
            widget_state_children: children.state_children.reborrow_mut(),
            widget_children: children.widget_children.reborrow_mut(),
            properties_children: children.properties_children.reborrow_mut(),
            tree_update,
            rebuild_all,
        };
        let mut node = build_access_node(&mut **item.widget, &mut ctx, scale_factor);
        let props = PropertiesRef {
            map: item.properties,
            default_map: default_properties.for_widget(item.widget.type_id()),
        };
        item.widget.accessibility(&mut ctx, &props, &mut node);

        let id: NodeId = ctx.widget_state.id.into();
        if ctx.global_state.trace.access {
            trace!("Built node {} with role={:?}", id.0, node.role());
        }
        ctx.tree_update.nodes.push((id, node));
    }

    item.state.request_accessibility = false;
    item.state.needs_accessibility = false;

    let id = item.state.id;
    let parent_state = item.state;
    crate::passes::recurse_on_children2(id, &**item.widget, children, |mut item, children| {
        // TODO - We don't skip updating stashed items because doing so
        // is error-prone. We may want to revisit that decision.
        build_accessibility_tree(
            global_state,
            default_properties,
            tree_update,
            item.reborrow_mut(),
            children,
            rebuild_all,
            None,
        );
        parent_state.merge_up(item.state);
    });
}

// --- MARK: BUILD NODE
fn build_access_node(
    widget: &mut dyn Widget,
    ctx: &mut AccessCtx<'_>,
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

// --- MARK: ROOT
/// See the [passes documentation](../doc/05_pass_system.md#render-passes).
pub(crate) fn run_accessibility_pass(root: &mut RenderRoot, scale_factor: f64) -> TreeUpdate {
    let _span = info_span!("accessibility").entered();

    let mut tree_update = TreeUpdate {
        nodes: vec![],
        tree: Some(Tree {
            root: root.window_node_id,
            toolkit_name: Some("Masonry".to_string()),
            toolkit_version: Some(env!("CARGO_PKG_VERSION").to_string()),
        }),
        focus: root
            .global_state
            .focused_widget
            .unwrap_or(root.root.id())
            .into(),
    };

    let (root_item, root_children) = root.widget_arena.get_mut(root.root.id());

    if root.rebuild_access_tree {
        debug!("Running ACCESSIBILITY pass with rebuild_all");
    }
    build_accessibility_tree(
        &mut root.global_state,
        &root.default_properties,
        &mut tree_update,
        root_item,
        root_children,
        root.rebuild_access_tree,
        Some(scale_factor),
    );
    root.rebuild_access_tree = false;

    // TODO: make root node type customizable to support Dialog/AlertDialog roles
    // (should go hand in hand with introducing support for modal windows?)
    let mut window_node = Node::new(Role::Window);
    window_node.set_children(vec![root.root.id().into()]);
    tree_update.nodes.push((root.window_node_id, window_node));

    tree_update
}
