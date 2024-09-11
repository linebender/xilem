// Copyright 2024 the Xilem Authors
// SPDX-License-Identifier: Apache-2.0

use std::collections::HashSet;

use cursor_icon::CursorIcon;
use tracing::{info_span, trace};

use crate::passes::{merge_state_up, recurse_on_children};
use crate::render_root::{RenderRoot, RenderRootSignal, RenderRootState};
use crate::tree_arena::ArenaMut;
use crate::{LifeCycle, LifeCycleCtx, StatusChange, Widget, WidgetId, WidgetState};

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

    let mut next_hovered_widget = if let Some(pos) = pointer_pos {
        // TODO - Apply scale?
        root.get_root_widget()
            .find_widget_at_pos(pos)
            .map(|widget| widget.id())
    } else {
        None
    };
    // If the pointer is captured, it can either hover its capture target or nothing.
    if let Some(capture_target) = root.state.pointer_capture_target {
        if next_hovered_widget != Some(capture_target) {
            next_hovered_widget = None;
        }
    }

    // "Hovered path" means the widget which is considered hovered, and all its parents.
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

    // TODO - Make sure widgets are iterated from the bottom up.
    // TODO - Document the iteration order for update_pointer pass.
    for widget_id in prev_hovered_path.iter().copied() {
        if root.widget_arena.has(widget_id)
            && root.widget_arena.get_state_mut(widget_id).item.is_hot
                != hovered_set.contains(&widget_id)
        {
            update_hovered_status_of(root, widget_id, &hovered_set);
        }
    }
    for widget_id in next_hovered_path.iter().copied() {
        if root.widget_arena.has(widget_id)
            && root.widget_arena.get_state_mut(widget_id).item.is_hot
                != hovered_set.contains(&widget_id)
        {
            update_hovered_status_of(root, widget_id, &hovered_set);
        }
    }

    // -- UPDATE CURSOR --

    // If the pointer is captured, its cursor always reflects the
    // capture target, even when not hovered.
    let cursor_source = root.state.pointer_capture_target.or(next_hovered_widget);

    let new_cursor = if let Some(cursor_source) = cursor_source {
        let (widget, state) = root.widget_arena.get_pair(cursor_source);
        state.item.cursor.unwrap_or(widget.item.get_cursor())
    } else {
        CursorIcon::Default
    };

    if root.state.cursor_icon != new_cursor {
        root.state
            .signal_queue
            .push_back(RenderRootSignal::SetCursor(new_cursor));
    }

    root.state.cursor_icon = new_cursor;
    root.state.hovered_path = next_hovered_path;

    // Pass root widget state to synthetic state create at beginning of pass
    root_state.merge_up(root.widget_arena.get_state_mut(root.root.id()).item);
}

// ----------------

fn update_disabled_for_widget(
    global_state: &mut RenderRootState,
    mut widget: ArenaMut<'_, Box<dyn Widget>>,
    mut state: ArenaMut<'_, WidgetState>,
    parent_disabled: bool,
) {
    let _span = widget.item.make_trace_span().entered();
    let id = state.item.id;

    let disabled = state.item.is_explicitly_disabled || parent_disabled;
    if !state.item.needs_update_disabled && disabled == state.item.is_disabled {
        return;
    }

    if disabled != state.item.is_disabled {
        let mut ctx = LifeCycleCtx {
            global_state,
            widget_state: state.item,
            widget_state_children: state.children.reborrow_mut(),
            widget_children: widget.children.reborrow_mut(),
        };
        widget
            .item
            .lifecycle(&mut ctx, &LifeCycle::DisabledChanged(disabled));
        state.item.is_disabled = disabled;
    }

    state.item.needs_update_disabled = false;

    if disabled && global_state.next_focused_widget == Some(id) {
        // This may get overwritten. That's ok, because either way the
        // focused widget, if there's one, won't be disabled.
        global_state.next_focused_widget = None;
    }

    let parent_state = state.item;
    recurse_on_children(
        id,
        widget.reborrow_mut(),
        state.children,
        |widget, mut state| {
            update_disabled_for_widget(global_state, widget, state.reborrow_mut(), disabled);
            parent_state.merge_up(state.item);
        },
    );
}

pub(crate) fn run_update_disabled_pass(root: &mut RenderRoot) {
    let _span = info_span!("update_disabled").entered();

    let (root_widget, root_state) = root.widget_arena.get_pair_mut(root.root.id());
    update_disabled_for_widget(&mut root.state, root_widget, root_state, false);
}

// ----------------

// This pass will update scroll positions in cases where a widget has requested to be
// scrolled into view (usually a textbox getting text events).
// Each parent that implements scrolling will update its scroll position to ensure the
// child is visible. (If the target area is larger than the parent, the parent will try
// to show the top left of that area.)
pub(crate) fn run_update_scroll_pass(root: &mut RenderRoot) {
    let _span = info_span!("update_scroll").entered();

    let scroll_request_targets = std::mem::take(&mut root.state.scroll_request_targets);
    for (target, rect) in scroll_request_targets {
        let mut target_rect = rect;

        run_targeted_update_pass(root, Some(target), |widget, ctx| {
            let event = LifeCycle::RequestPanToChild(rect);
            widget.lifecycle(ctx, &event);

            // TODO - We should run the compose method after this, so
            // translations are updated and the rect passed to parents
            // is more accurate.

            let state = &ctx.widget_state;
            target_rect = target_rect + state.translation + state.origin.to_vec2();
        });
    }
}
