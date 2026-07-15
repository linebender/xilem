// Copyright 2024 the Xilem Authors
// SPDX-License-Identifier: Apache-2.0

use accesskit::{Node, NodeId, Role, Tree, TreeId, TreeUpdate};
use kurbo::Rect;
use tracing::{info_span, trace};
use tree_arena::ArenaMut;

use crate::app::{RenderRoot, RenderRootState};
use crate::core::{
    AccessCtx, DefaultProperties, PropertiesRef, PropertyArena, Widget, WidgetArenaNode, WidgetId,
};
use crate::passes::{enter_span_if, recurse_on_children};

// --- MARK: BUILD TREE
fn build_accessibility_tree(
    global_state: &mut RenderRootState,
    default_properties: &DefaultProperties,
    property_arena: &PropertyArena,
    tree_update: &mut TreeUpdate,
    node: ArenaMut<'_, WidgetArenaNode>,
    scale_factor: Option<f64>,
    parent_hidden: bool,
) {
    let mut children = node.children;
    let widget = &mut *node.item.widget;
    let state = &mut node.item.state;
    let properties = &mut node.item.properties;
    let class_set = &node.item.class_set;
    let id = state.id;
    let hidden = parent_hidden || state.accessibility_hidden;
    let _span = enter_span_if(global_state.trace.access, state);

    if !state.needs_accessibility {
        return;
    }

    if state.request_accessibility && !state.is_stashed && !hidden {
        if global_state.trace.access {
            trace!(
                "Building accessibility node for widget '{}' {}",
                widget.short_type_name(),
                id,
            );
        }

        let stack = property_arena.get(state.property_stack_id, widget.type_id());
        let mut ctx = AccessCtx {
            global_state,
            widget_state: state,
            children: children.reborrow_mut(),
            tree_update,
        };
        let mut node = build_access_node(widget, &mut ctx, scale_factor);
        let props = PropertiesRef {
            local: properties,
            default_map: default_properties.for_widget(widget.type_id()),
            stack,
            class_set,
        };
        widget.accessibility(&mut ctx, &props, &mut node);

        let id: NodeId = ctx.widget_state.id.into();
        if ctx.global_state.trace.access {
            trace!("Built node {} with role={:?}", id.0, node.role());
        }
        ctx.tree_update.nodes.push((id, node));
    }

    state.request_accessibility = false;
    state.needs_accessibility = false;

    let parent_state = state;
    recurse_on_children(id, widget, children, |mut node| {
        build_accessibility_tree(
            global_state,
            default_properties,
            property_arena,
            tree_update,
            node.reborrow_mut(),
            None,
            hidden,
        );
        parent_state.merge_up(&mut node.item.state);
    });
}

// --- MARK: BUILD NODE
fn build_access_node(
    widget: &mut dyn Widget,
    ctx: &mut AccessCtx<'_>,
    scale_factor: Option<f64>,
) -> Node {
    let mut node = Node::new(widget.accessibility_role());
    node.set_bounds(to_accesskit_rect(ctx.widget_state.border_box()));

    let mut local_transform = ctx.widget_state.compose_local_transform();

    // TODO - Remove once Masonry uses physical coordinates.
    // See https://github.com/linebender/xilem/issues/1264
    if let Some(scale_factor) = scale_factor {
        local_transform = local_transform.pre_scale(scale_factor);
    }
    node.set_transform(accesskit::Affine::new(local_transform.as_coeffs()));

    fn is_child_hidden(ctx: &mut AccessCtx<'_>, id: WidgetId) -> bool {
        let state = &ctx
            .children
            .find(id)
            .expect("is_child_hidden: child not found")
            .item
            .state;
        state.is_stashed || state.accessibility_hidden
    }

    node.set_children(
        widget
            .children_ids()
            .iter()
            .copied()
            .filter(|id| !is_child_hidden(ctx, *id))
            .map(|id| id.into())
            .collect::<Vec<NodeId>>(),
    );

    if ctx.is_stashed() {
        debug_panic!("build_access_node called for stashed widget");
    }

    // Note - The values returned by these methods can be modified by other passes.
    // When that happens, the other pass should set flags to request an accessibility pass.
    if ctx.is_disabled() {
        node.set_disabled();
    }
    if ctx.widget_state.clip_path.is_some() {
        node.set_clips_children();
    }
    if ctx.accepts_focus() && !ctx.is_disabled() {
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

fn is_accessibility_hidden(root: &RenderRoot, mut id: WidgetId) -> bool {
    loop {
        if root
            .widget_arena
            .get_node(id)
            .item
            .state
            .accessibility_hidden
        {
            return true;
        }
        let Some(parent_id) = root.widget_arena.parent_of(id) else {
            return false;
        };
        id = parent_id;
    }
}

// --- MARK: ROOT
/// See the [passes documentation](crate::doc::pass_system#render-passes).
pub(crate) fn run_accessibility_pass(root: &mut RenderRoot, scale_factor: f64) -> TreeUpdate {
    let _span = info_span!("accessibility").entered();
    let focus = root
        .global_state
        .focused_widget
        .filter(|id| !is_accessibility_hidden(root, *id))
        .map(Into::into)
        .unwrap_or(root.global_state.window_node_id);

    let mut tree_update = TreeUpdate {
        tree_id: TreeId::ROOT,
        nodes: vec![],
        tree: Some(Tree {
            root: root.global_state.window_node_id,
            toolkit_name: Some("Masonry".to_string()),
            toolkit_version: Some(env!("CARGO_PKG_VERSION").to_string()),
        }),
        focus,
    };

    let root_id = root.root_id();
    let root_hidden = root.root_state().accessibility_hidden;
    let root_node = root.widget_arena.get_node_mut(root_id);

    build_accessibility_tree(
        &mut root.global_state,
        &root.property_arena.default_properties,
        &root.property_arena,
        &mut tree_update,
        root_node,
        Some(scale_factor),
        false,
    );

    // TODO: make root node type customizable to support Dialog/AlertDialog roles
    // (should go hand in hand with introducing support for modal windows?)
    let mut window_node = Node::new(Role::Window);
    if !root_hidden {
        window_node.set_children(vec![root_id.into()]);
    }
    tree_update
        .nodes
        .push((root.global_state.window_node_id, window_node));

    tree_update
}
