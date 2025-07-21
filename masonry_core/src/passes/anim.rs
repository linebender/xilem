// Copyright 2024 the Xilem Authors
// SPDX-License-Identifier: Apache-2.0

use tracing::info_span;
use tree_arena::ArenaMut;

use crate::app::{RenderRoot, RenderRootState};
use crate::core::{
    DefaultProperties, PropertiesMut, UpdateCtx, Widget, WidgetArenaMut, WidgetState,
};
use crate::passes::{enter_span_if, recurse_on_children};
use crate::util::AnyMap;

// --- MARK: UPDATE ANIM
fn update_anim_for_widget(
    global_state: &mut RenderRootState,
    default_properties: &DefaultProperties,
    widget: ArenaMut<'_, Box<dyn Widget>>,
    mut state: ArenaMut<'_, WidgetState>,
    properties: ArenaMut<'_, AnyMap>,
    elapsed_ns: u64,
) {
    let _span = enter_span_if(global_state.trace.anim, state.reborrow());
    let mut children = WidgetArenaMut {
        widget_children: widget.children,
        widget_state_children: state.children,
        properties_children: properties.children,
    };
    let widget = &mut **widget.item;
    let state = state.item;
    let properties = properties.item;

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
        };
        let mut props = PropertiesMut {
            map: properties,
            default_map: default_properties.for_widget(widget.type_id()),
        };
        widget.on_anim_frame(&mut ctx, &mut props, elapsed_ns);
    }

    let id = state.id;
    let parent_state = state;
    recurse_on_children(id, widget, children, |widget, mut state, properties| {
        update_anim_for_widget(
            global_state,
            default_properties,
            widget,
            state.reborrow_mut(),
            properties,
            elapsed_ns,
        );
        parent_state.merge_up(state.item);
    });
}

// TODO - switch anim frames to being about age / an absolute timestamp
// instead of time elapsed.
// (this will help in cases where we want to skip anim frames)

/// Run the animation pass.
///
/// See the [passes documentation](../doc/05_pass_system.md#animation-pass).
pub(crate) fn run_update_anim_pass(root: &mut RenderRoot, elapsed_ns: u64) {
    let _span = info_span!("update_anim").entered();

    let (root_widget, mut root_state, root_properties) =
        root.widget_arena.get_all_mut(root.root.id());
    update_anim_for_widget(
        &mut root.global_state,
        &root.default_properties,
        root_widget,
        root_state.reborrow_mut(),
        root_properties,
        elapsed_ns,
    );
}
