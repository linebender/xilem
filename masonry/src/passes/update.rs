// Copyright 2024 the Xilem Authors
// SPDX-License-Identifier: Apache-2.0

use std::collections::HashSet;

use cursor_icon::CursorIcon;
use tracing::trace;

use crate::passes::merge_state_up;
use crate::render_root::{RenderRoot, RenderRootSignal};
use crate::{LifeCycleCtx, StatusChange, Widget, WidgetId, WidgetState};

fn get_id_path(root: &RenderRoot, widget_id: Option<WidgetId>) -> Vec<WidgetId> {
    let Some(widget_id) = widget_id else {
        return Vec::new();
    };

    root.widget_arena
        .widget_states
        .get_id_path(widget_id.to_raw())
        .iter()
        .map(|&id| WidgetId(id.try_into().unwrap()))
        .collect()
}

// TODO - Replace LifecycleCtx with UpdateCtx
fn run_targeted_update_pass(
    root: &mut RenderRoot,
    target: Option<WidgetId>,
    mut pass_fn: impl FnMut(&mut dyn Widget, &mut LifeCycleCtx),
) {
    let mut current_id = target;
    while let Some(widget_id) = current_id {
        let parent_id = root.widget_arena.parent_of(widget_id);
        let (widget_mut, state_mut) = root.widget_arena.get_pair_mut(widget_id);

        let mut ctx = LifeCycleCtx {
            global_state: &mut root.state,
            widget_state: state_mut.item,
            widget_state_children: state_mut.children,
            widget_children: widget_mut.children,
        };
        pass_fn(widget_mut.item, &mut ctx);

        merge_state_up(&mut root.widget_arena, widget_id);
        current_id = parent_id;
    }
}

pub(crate) fn run_update_pointer_pass(root: &mut RenderRoot, root_state: &mut WidgetState) {
    let pointer_pos = root.last_mouse_pos.map(|pos| (pos.x, pos.y).into());

    // -- UPDATE HOVERED WIDGETS --

    let hovered_widget = root.state.hovered_path.last().copied();
    let mut next_hovered_widget = if let Some(pos) = pointer_pos {
        // TODO - Apply scale?
        root.get_root_widget()
            .find_widget_at_pos(pos)
            .map(|widget| widget.id())
    } else {
        None
    };
    if let Some(capture_target) = root.state.pointer_capture_target {
        if next_hovered_widget != Some(capture_target) {
            next_hovered_widget = None;
        }
    }

    let prev_hovered_path = std::mem::take(&mut root.state.hovered_path);
    let next_hovered_path = get_id_path(root, next_hovered_widget);

    let mut hovered_set = HashSet::new();
    for widget_id in &next_hovered_path {
        hovered_set.insert(*widget_id);
    }

    trace!("prev_hovered_path: {:?}", prev_hovered_path);
    trace!("next_hovered_path: {:?}", next_hovered_path);

    // This algorithm is written to be resilient to future changes like reparenting and multiple
    // cursors. In theory it's O(Depth² * CursorCount) in the worst case, which isn't too bad
    // (cursor count is usually 1 or 2, depth is usually small), but in practice it's virtually
    // always O(Depth * CursorCount) because we only need to update the hovered status of the
    // widgets that changed.
    // The above assumes that accessing the widget tree is O(1) for simplicity.

    fn update_hovered_status_of(
        root: &mut RenderRoot,
        widget_id: WidgetId,
        hovered_set: &HashSet<WidgetId>,
    ) {
        run_targeted_update_pass(root, Some(widget_id), |widget, ctx| {
            let is_hot = hovered_set.contains(&ctx.widget_id());

            if ctx.widget_state.is_hot != is_hot {
                widget.on_status_change(ctx, &StatusChange::HotChanged(is_hot));
            }
            ctx.widget_state.is_hot = is_hot;
        });
    }

    // TODO - Make sure widgets are iterated from the bottom up
    for widget_id in &prev_hovered_path {
        if root
            .widget_arena
            .widget_states
            .find(widget_id.to_raw())
            .is_some()
            && root.widget_arena.get_state_mut(*widget_id).item.is_hot
                != hovered_set.contains(widget_id)
        {
            update_hovered_status_of(root, *widget_id, &hovered_set);
        }
    }
    for widget_id in &next_hovered_path {
        if root
            .widget_arena
            .widget_states
            .find(widget_id.to_raw())
            .is_some()
            && root.widget_arena.get_state_mut(*widget_id).item.is_hot
                != hovered_set.contains(widget_id)
        {
            update_hovered_status_of(root, *widget_id, &hovered_set);
        }
    }

    root.state.hovered_path = next_hovered_path;

    // -- UPDATE CURSOR --
    // TODO - Rewrite more cleanly
    let cursor_changed = next_hovered_widget
        .is_some_and(|id| root.widget_arena.get_state_mut(id).item.cursor_changed);
    if hovered_widget != next_hovered_widget || cursor_changed {
        let cursor;
        if let Some(capture_target) = root.state.pointer_capture_target {
            let widget = root.widget_arena.get_widget(capture_target).item;
            cursor = widget.get_cursor();
        } else if let Some(next_hovered_widget) = next_hovered_widget {
            let widget = root.widget_arena.get_widget(next_hovered_widget).item;
            cursor = widget.get_cursor();
        } else {
            cursor = CursorIcon::Default;
        }
        // TODO - Add methods and `into()` impl to make this more concise.
        root.state
            .signal_queue
            .push_back(RenderRootSignal::SetCursor(cursor));

        if let Some(next_hovered_widget) = next_hovered_widget {
            root.widget_arena
                .get_state_mut(next_hovered_widget)
                .item
                .cursor_changed = false;
        }
    }

    // Pass root widget state to synthetic state create at beginning of pass
    root_state.merge_up(root.widget_arena.get_state_mut(root.root.id()).item);
}
