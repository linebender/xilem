// Copyright 2024 the Xilem Authors
// SPDX-License-Identifier: Apache-2.0

use tracing::info_span;
use tree_arena::ArenaMut;

use crate::app::{RenderRoot, RenderRootState};
use crate::core::{DefaultProperties, PropertiesMut, UpdateCtx, WidgetArenaNode};
use crate::passes::{enter_span_if, recurse_on_children};

// --- MARK: UPDATE ANIM
fn update_anim_for_widget(
    global_state: &mut RenderRootState,
    default_properties: &DefaultProperties,
    node: ArenaMut<'_, WidgetArenaNode>,
    elapsed_ns: u64,
) {
    let mut children = node.children;
    let widget = &mut *node.item.widget;
    let state = &mut node.item.state;
    let properties = &mut node.item.properties;
    let id = state.id;
    let _span = enter_span_if(global_state.trace.anim, state);

    if !state.needs_anim {
        return;
    }
    state.needs_anim = false;

    // Most passes reset their `needs` and `request` flags after the call to
    // the widget method, but it's valid and expected for `request_anim` to be
    // set in response to `AnimFrame`.
    if state.request_anim {
        state.request_anim = false;
        let mut ctx = UpdateCtx {
            global_state,
            widget_state: state,
            children: children.reborrow_mut(),
            default_properties,
            ancestors: None,
        };
        let mut props = PropertiesMut {
            map: properties,
            default_map: default_properties.for_widget(widget.type_id()),
        };
        widget.on_anim_frame(&mut ctx, &mut props, elapsed_ns);
    }

    let parent_state = state;
    recurse_on_children(id, widget, children, |mut node| {
        update_anim_for_widget(
            global_state,
            default_properties,
            node.reborrow_mut(),
            elapsed_ns,
        );
        parent_state.merge_up(&mut node.item.state);
    });
}

// TODO - switch anim frames to being about age / an absolute timestamp
// instead of time elapsed.
// (this will help in cases where we want to skip anim frames)

/// Run the animation pass.
///
/// See the [passes documentation](crate::doc::pass_system#animation-pass).
pub(crate) fn run_update_anim_pass(root: &mut RenderRoot, elapsed_ns: u64) {
    let _span = info_span!("update_anim").entered();

    let root_node = root.widget_arena.get_node_mut(root.root_id());
    update_anim_for_widget(
        &mut root.global_state,
        &root.default_properties,
        root_node,
        elapsed_ns,
    );
}
