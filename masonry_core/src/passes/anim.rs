// Copyright 2024 the Xilem Authors
// SPDX-License-Identifier: Apache-2.0

use tracing::info_span;

use crate::app::{RenderRoot, RenderRootState};
use crate::core::{DefaultProperties, PropertiesMut, UpdateCtx, WidgetArenaMut, WidgetItemMut};
use crate::passes::enter_span_if;

// --- MARK: UPDATE ANIM
fn update_anim_for_widget(
    global_state: &mut RenderRootState,
    default_properties: &DefaultProperties,
    item: WidgetItemMut<'_>,
    mut children: WidgetArenaMut<'_>,
    elapsed_ns: u64,
) {
    let _span = enter_span_if(global_state.trace.anim, &**item.widget, item.state.id);
    if !item.state.needs_anim {
        return;
    }
    item.state.needs_anim = false;

    // Most passes reset their `needs` and `request` flags after the call to
    // the widget method, but it's valid and expected for `request_anim` to be
    // set in response to `AnimFrame`.
    if item.state.request_anim {
        item.state.request_anim = false;
        let mut ctx = UpdateCtx {
            global_state,
            widget_state: item.state,
            widget_state_children: children.state_children.reborrow_mut(),
            widget_children: children.widget_children.reborrow_mut(),
            properties_children: children.properties_children.reborrow_mut(),
        };
        let mut props = PropertiesMut {
            map: item.properties,
            default_map: default_properties.for_widget(item.widget.type_id()),
        };
        item.widget.on_anim_frame(&mut ctx, &mut props, elapsed_ns);
    }

    let id = item.state.id;
    let parent_state = item.state;
    crate::passes::recurse_on_children2(id, &**item.widget, children, |mut item, children| {
        update_anim_for_widget(
            global_state,
            default_properties,
            item.reborrow_mut(),
            children,
            elapsed_ns,
        );
        parent_state.merge_up(item.state);
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

    let (root_item, root_children) = root.widget_arena.get_mut(root.root.id());
    update_anim_for_widget(
        &mut root.global_state,
        &root.default_properties,
        root_item,
        root_children,
        elapsed_ns,
    );
}
