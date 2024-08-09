// Copyright 2024 the Xilem Authors
// SPDX-License-Identifier: Apache-2.0

use std::collections::HashSet;

use cursor_icon::CursorIcon;
use tracing::trace;

use crate::{
    render_root::{RenderRoot, RenderRootSignal, WidgetArena},
    tree_arena::{ArenaMut, ArenaMutChildren},
    LifeCycleCtx, StatusChange, Widget, WidgetId, WidgetState,
};

// References shared by all passes
struct PassCtx<'a> {
    pub(crate) widget_state: ArenaMut<'a, WidgetState>,
    pub(crate) widget_children: ArenaMutChildren<'a, Box<dyn Widget>>,
}

impl<'a> PassCtx<'a> {
    fn parent(&self) -> Option<WidgetId> {
        let parent_id = self.widget_state.parent_id?;
        let parent_id = parent_id.try_into().unwrap();
        Some(WidgetId(parent_id))
    }
}

// TODO - Merge copy-pasted code
fn merge_state_up(arena: &mut WidgetArena, widget_id: WidgetId, root_state: &mut WidgetState) {
    let parent_id = get_widget_mut(arena, widget_id).1.parent();

    let Some(parent_id) = parent_id else {
        // We've reached the root
        let child_state_mut = arena.widget_states.find_mut(widget_id.to_raw()).unwrap();
        root_state.merge_up(child_state_mut.item);
        return;
    };

    let mut parent_state_mut = arena.widget_states.find_mut(parent_id.to_raw()).unwrap();
    let child_state_mut = parent_state_mut
        .children
        .get_child_mut(widget_id.to_raw())
        .unwrap();

    parent_state_mut.item.merge_up(child_state_mut.item);
}

fn get_widget_mut(arena: &mut WidgetArena, id: WidgetId) -> (&mut dyn Widget, PassCtx<'_>) {
    let state_mut = arena
        .widget_states
        .find_mut(id.to_raw())
        .expect("widget state not found in arena");
    let widget_mut = arena
        .widgets
        .find_mut(id.to_raw())
        .expect("widget not found in arena");

    // Box<dyn Widget> -> &dyn Widget
    // Without this step, the type of `WidgetRef::widget` would be
    // `&Box<dyn Widget> as &dyn Widget`, which would be an additional layer
    // of indirection.
    let widget = widget_mut.item;
    let widget: &mut dyn Widget = &mut **widget;

    (
        widget,
        PassCtx {
            widget_state: state_mut,
            widget_children: widget_mut.children,
        },
    )
}

// TODO - Replace LifecycleCtx with UpdateCtx
pub(crate) fn run_targeted_update_pass(
    root: &mut RenderRoot,
    root_state: &mut WidgetState,
    target: Option<WidgetId>,
    pass_fn: impl FnMut(&mut dyn Widget, &mut LifeCycleCtx),
) {
    let mut pass_fn = pass_fn;

    let mut target_widget_id = target;
    while let Some(widget_id) = target_widget_id {
        let (widget, pass_ctx) = get_widget_mut(&mut root.widget_arena, widget_id);
        let parent_id = pass_ctx.parent();

        let mut ctx = LifeCycleCtx {
            global_state: &mut root.state,
            widget_state: pass_ctx.widget_state.item,
            widget_state_children: pass_ctx.widget_state.children,
            widget_children: pass_ctx.widget_children,
        };
        pass_fn(widget, &mut ctx);

        merge_state_up(&mut root.widget_arena, widget_id, root_state);
        target_widget_id = parent_id;
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
    // (cursor count is usually 1 or 2, depth is usually small), but in practice it's almost
    // always O(Depth * CursorCount) because we only need to update the hovered status of the
    // widgets that changed.

    // TODO - Make sure widgets are iterated from the bottom up
    for widget_id in &prev_hovered_path {
        if root
            .widget_arena
            .widget_states
            .find(widget_id.to_raw())
            .is_some()
            && root.widget_arena.get_state_mut(*widget_id).is_hot != hovered_set.contains(widget_id)
        {
            update_hovered_status_of(root, root_state, *widget_id, &hovered_set);
        }
    }
    for widget_id in &next_hovered_path {
        if root
            .widget_arena
            .widget_states
            .find(widget_id.to_raw())
            .is_some()
            && root.widget_arena.get_state_mut(*widget_id).is_hot != hovered_set.contains(widget_id)
        {
            update_hovered_status_of(root, root_state, *widget_id, &hovered_set);
        }
    }

    root.state.hovered_path = next_hovered_path;

    // -- UPDATE CURSOR --
    // TODO - Rewrite more cleanly
    let cursor_changed =
        next_hovered_widget.is_some_and(|id| root.widget_arena.get_state_mut(id).cursor_changed);
    if hovered_widget != next_hovered_widget || cursor_changed {
        let cursor;
        if let Some(capture_target) = root.state.pointer_capture_target {
            let (widget, _) = get_widget_mut(&mut root.widget_arena, capture_target);
            cursor = widget.get_cursor();
        } else if let Some(next_hovered_widget) = next_hovered_widget {
            let (widget, _) = get_widget_mut(&mut root.widget_arena, next_hovered_widget);
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
                .cursor_changed = false;
        }
    }
}

fn update_hovered_status_of(
    root: &mut RenderRoot,
    root_state: &mut WidgetState,
    widget_id: WidgetId,
    hovered_set: &HashSet<WidgetId>,
) {
    run_targeted_update_pass(root, root_state, Some(widget_id), |widget, ctx| {
        let is_hot = hovered_set.contains(&ctx.widget_id());

        if ctx.widget_state.is_hot != is_hot {
            widget.on_status_change(ctx, &StatusChange::HotChanged(is_hot));
        }
        ctx.widget_state.is_hot = is_hot;
    });
}

fn get_id_path(root: &mut RenderRoot, widget_id: Option<WidgetId>) -> Vec<WidgetId> {
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
