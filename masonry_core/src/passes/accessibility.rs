// Copyright 2024 the Xilem Authors
// SPDX-License-Identifier: Apache-2.0

use accesskit::{Node, NodeId, Role, Tree, TreeUpdate};
use tracing::{info_span, trace};
use tree_arena::ArenaMut;
use vello::kurbo::Rect;

use crate::app::{RenderRoot, RenderRootState};
use crate::core::{AccessCtx, DefaultProperties, PropertiesRef, Widget, WidgetArenaNode, WidgetId};
use crate::passes::{enter_span_if, recurse_on_children};

// --- MARK: BUILD TREE
fn build_accessibility_tree(
    global_state: &mut RenderRootState,
    default_properties: &DefaultProperties,
    tree_update: &mut TreeUpdate,
    node: ArenaMut<'_, WidgetArenaNode>,
    scale_factor: Option<f64>,
) {
    let mut children = node.children;
    let widget = &mut *node.item.widget;
    let state = &mut node.item.state;
    let properties = &mut node.item.properties;
    let id = state.id;
    let _span = enter_span_if(global_state.trace.access, state);

    if !state.needs_accessibility {
        return;
    }

    if state.request_accessibility && !state.is_stashed {
        if global_state.trace.access {
            trace!(
                "Building accessibility node for widget '{}' {}",
                widget.short_type_name(),
                id,
            );
        }

        let mut ctx = AccessCtx {
            global_state,
            widget_state: state,
            children: children.reborrow_mut(),
            tree_update,
        };
        let mut node = build_access_node(widget, &mut ctx, scale_factor);
        let props = PropertiesRef {
            map: properties,
            default_map: default_properties.for_widget(widget.type_id()),
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
            tree_update,
            node.reborrow_mut(),
            None,
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
    node.set_bounds(to_accesskit_rect(ctx.widget_state.size().to_rect()));

    let local_translation = ctx.widget_state.scroll_translation + ctx.widget_state.origin.to_vec2();
    let mut local_transform = ctx.widget_state.transform.then_translate(local_translation);

    // TODO - Remove once Masonry uses physical coordinates.
    // See https://github.com/linebender/xilem/issues/1264
    if let Some(scale_factor) = scale_factor {
        local_transform = local_transform.pre_scale(scale_factor);
    }
    node.set_transform(accesskit::Affine::new(local_transform.as_coeffs()));

    fn is_child_stashed(ctx: &mut AccessCtx<'_>, id: WidgetId) -> bool {
        ctx.children
            .find(id)
            .expect("is_child_stashed: child not found")
            .item
            .state
            .is_stashed
    }

    node.set_children(
        widget
            .children_ids()
            .iter()
            .copied()
            .filter(|id| !is_child_stashed(ctx, *id))
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
    if ctx.widget_state.clips_contents {
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

// --- MARK: ROOT
/// See the [passes documentation](crate::doc::pass_system#render-passes).
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
            .map(Into::into)
            .unwrap_or(root.window_node_id),
    };

    let root_node = root.widget_arena.get_node_mut(root.root_id());

    build_accessibility_tree(
        &mut root.global_state,
        &root.default_properties,
        &mut tree_update,
        root_node,
        Some(scale_factor),
    );

    // TODO: make root node type customizable to support Dialog/AlertDialog roles
    // (should go hand in hand with introducing support for modal windows?)
    let mut window_node = Node::new(Role::Window);
    window_node.set_children(vec![root.root_id().into()]);
    tree_update.nodes.push((root.window_node_id, window_node));

    tree_update
}
