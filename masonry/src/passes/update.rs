// Copyright 2024 the Xilem Authors
// SPDX-License-Identifier: Apache-2.0

use std::collections::HashSet;

use cursor_icon::CursorIcon;
use tracing::{info_span, trace};

use crate::passes::event::run_on_pointer_event_pass;
use crate::passes::{merge_state_up, recurse_on_children};
use crate::render_root::{RenderRoot, RenderRootSignal, RenderRootState};
use crate::tree_arena::ArenaMut;
use crate::{
    LifeCycle, LifeCycleCtx, PointerEvent, RegisterCtx, StatusChange, Widget, WidgetId, WidgetState,
};

// --- MARK: HELPERS ---
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

// TODO - Replace LifecycleCtx with UpdateCtx
fn run_single_update_pass(
    root: &mut RenderRoot,
    target: Option<WidgetId>,
    mut pass_fn: impl FnMut(&mut dyn Widget, &mut LifeCycleCtx),
) {
    if let Some(widget_id) = target {
        let (widget_mut, state_mut) = root.widget_arena.get_pair_mut(widget_id);

        let mut ctx = LifeCycleCtx {
            global_state: &mut root.state,
            widget_state: state_mut.item,
            widget_state_children: state_mut.children,
            widget_children: widget_mut.children,
        };
        pass_fn(widget_mut.item, &mut ctx);
    }

    let mut current_id = target;
    while let Some(widget_id) = current_id {
        merge_state_up(&mut root.widget_arena, widget_id);
        current_id = root.widget_arena.parent_of(widget_id);
    }
}

// --- MARK: UPDATE TREE ---
fn update_widget_tree(
    global_state: &mut RenderRootState,
    mut widget: ArenaMut<'_, Box<dyn Widget>>,
    mut state: ArenaMut<'_, WidgetState>,
) {
    let _span = widget.item.make_trace_span().entered();
    let id = state.item.id;

    if !state.item.children_changed {
        return;
    }
    state.item.children_changed = false;

    {
        let mut ctx = RegisterCtx {
            widget_state_children: state.children.reborrow_mut(),
            widget_children: widget.children.reborrow_mut(),
            #[cfg(debug_assertions)]
            registered_ids: Vec::new(),
        };
        // The widget will call `RegisterCtx::register_child` on all its children,
        // which will add the new widgets to the arena.
        widget.item.register_children(&mut ctx);

        #[cfg(debug_assertions)]
        {
            let children_ids = widget.item.children_ids();
            for child_id in ctx.registered_ids {
                if !children_ids.contains(&child_id) {
                    panic!(
                        "Error in '{}' #{}: method register_children() called \
                        RegisterCtx::register_child() on child #{}, which isn't \
                        in the list returned by children_ids()",
                        widget.item.short_type_name(),
                        id.to_raw(),
                        child_id.to_raw()
                    );
                }
            }
        }

        #[cfg(debug_assertions)]
        for child_id in widget.item.children_ids() {
            if widget.children.get_child(child_id.to_raw()).is_none() {
                panic!(
                    "Error in '{}' #{}: method register_children() did not call \
                    RegisterCtx::register_child() on child #{} returned by children_ids()",
                    widget.item.short_type_name(),
                    id.to_raw(),
                    child_id.to_raw()
                );
            }
        }
    }

    if state.item.is_new {
        let mut ctx = LifeCycleCtx {
            global_state,
            widget_state: state.item,
            widget_state_children: state.children.reborrow_mut(),
            widget_children: widget.children.reborrow_mut(),
        };
        widget.item.lifecycle(&mut ctx, &LifeCycle::WidgetAdded);
        trace!(
            "{} received LifeCycle::WidgetAdded",
            widget.item.short_type_name()
        );
        state.item.accepts_pointer_interaction = widget.item.accepts_pointer_interaction();
        state.item.accepts_focus = widget.item.accepts_focus();
        state.item.accepts_text_input = widget.item.accepts_text_input();
        state.item.is_new = false;
    }

    // We can recurse on this widget's children, because they have already been added
    // to the arena above.
    let parent_state = state.item;
    recurse_on_children(
        id,
        widget.reborrow_mut(),
        state.children,
        |widget, mut state| {
            update_widget_tree(global_state, widget, state.reborrow_mut());
            parent_state.merge_up(state.item);
        },
    );
}

pub(crate) fn run_update_widget_tree_pass(root: &mut RenderRoot) {
    let _span = info_span!("update_new_widgets").entered();

    if root.root.incomplete() {
        let mut ctx = RegisterCtx {
            widget_state_children: root.widget_arena.widget_states.root_token_mut(),
            widget_children: root.widget_arena.widgets.root_token_mut(),
            #[cfg(debug_assertions)]
            registered_ids: Vec::new(),
        };
        ctx.register_child(&mut root.root);
    }

    let (root_widget, mut root_state) = root.widget_arena.get_pair_mut(root.root.id());
    update_widget_tree(&mut root.state, root_widget, root_state.reborrow_mut());
}

// ----------------

// --- MARK: UPDATE DISABLED ---
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
        state.item.update_focus_chain = true;
    }

    state.item.needs_update_disabled = false;

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

// TODO - Document the stashed pass.
// *Stashed* is for widgets that are no longer "part of the graph". So they can't get keyboard events, don't get painted, etc, but should keep some state.
// The stereotypical use case would be the contents of hidden tabs in a "tab group" widget.
// Scrolled-out widgets are *not* stashed.

// --- MARK: UPDATE STASHED ---
fn update_stashed_for_widget(
    global_state: &mut RenderRootState,
    mut widget: ArenaMut<'_, Box<dyn Widget>>,
    mut state: ArenaMut<'_, WidgetState>,
    parent_stashed: bool,
) {
    let _span = widget.item.make_trace_span().entered();
    let id = state.item.id;

    let stashed = state.item.is_explicitly_stashed || parent_stashed;
    if !state.item.needs_update_stashed && stashed == state.item.is_stashed {
        return;
    }

    if stashed != state.item.is_stashed {
        let mut ctx = LifeCycleCtx {
            global_state,
            widget_state: state.item,
            widget_state_children: state.children.reborrow_mut(),
            widget_children: widget.children.reborrow_mut(),
        };
        widget
            .item
            .lifecycle(&mut ctx, &LifeCycle::StashedChanged(stashed));
        state.item.is_stashed = stashed;
        state.item.update_focus_chain = true;
        // Note: We don't need request_repaint because stashing doesn't actually change
        // how widgets are painted, only how the Scenes they create are composed.
        state.item.needs_paint = true;
        state.item.needs_accessibility = true;
        // TODO - Remove once accessibility can be composed, same as above.
        state.item.request_accessibility = true;
        // A stashed child doesn't need layout. We assumed that a child that just got
        // un-stashed will need relayout.
        // TODO - Handle this interaction more elegantly.
        state.item.needs_layout = !stashed;
        state.item.request_layout = !stashed;
    }

    state.item.needs_update_stashed = false;

    let parent_state = state.item;
    recurse_on_children(
        id,
        widget.reborrow_mut(),
        state.children,
        |widget, mut state| {
            update_stashed_for_widget(global_state, widget, state.reborrow_mut(), stashed);
            parent_state.merge_up(state.item);
        },
    );
}

pub(crate) fn run_update_stashed_pass(root: &mut RenderRoot) {
    let _span = info_span!("update_stashed").entered();

    let (root_widget, root_state) = root.widget_arena.get_pair_mut(root.root.id());
    update_stashed_for_widget(&mut root.state, root_widget, root_state, false);
}

// ----------------

// --- MARK: UPDATE FOCUS CHAIN ---

// TODO https://github.com/linebender/xilem/issues/376 - Some implicit invariants:
// - A widget only receives BuildFocusChain if none of its parents are hidden.

// TODO - This logic was copy-pasted from WidgetPod code and may need to be refactored.
// It doesn't quite behave like other update passes (for instance, some code runs after
// recurse_on_children), and some design decisions inherited from Druid should be reconsidered.
fn update_focus_chain_for_widget(
    global_state: &mut RenderRootState,
    mut widget: ArenaMut<'_, Box<dyn Widget>>,
    state: ArenaMut<'_, WidgetState>,
    parent_focus_chain: &mut Vec<WidgetId>,
) {
    let _span = widget.item.make_trace_span().entered();
    let id = state.item.id;

    if !state.item.update_focus_chain {
        return;
    }

    // Replace has_focus to check if the value changed in the meantime
    state.item.has_focus = global_state.focused_widget == Some(id);
    let had_focus = state.item.has_focus;

    state.item.focus_chain.clear();
    if state.item.accepts_focus {
        state.item.focus_chain.push(id);
    }
    state.item.update_focus_chain = false;

    let parent_state = &mut *state.item;
    recurse_on_children(
        id,
        widget.reborrow_mut(),
        state.children,
        |widget, mut state| {
            update_focus_chain_for_widget(
                global_state,
                widget,
                state.reborrow_mut(),
                &mut parent_state.focus_chain,
            );
            parent_state.merge_up(state.item);
        },
    );

    if !state.item.is_disabled {
        parent_focus_chain.extend(&state.item.focus_chain);
    }

    // had_focus is the old focus value. state.has_focus was replaced with parent_ctx.is_focused().
    // Therefore if had_focus is true but state.has_focus is false then the widget which is
    // currently focused is not part of the functional tree anymore
    // (Lifecycle::BuildFocusChain.should_propagate_to_hidden() is false!) and should
    // resign the focus.
    if had_focus && !state.item.has_focus {
        // Not sure about this logic, might remove
        global_state.next_focused_widget = None;
    }
    state.item.has_focus = had_focus;
}

pub(crate) fn run_update_focus_chain_pass(root: &mut RenderRoot) {
    let _span = info_span!("update_focus_chain").entered();
    let mut dummy_focus_chain = Vec::new();

    let (root_widget, mut root_state) = root.widget_arena.get_pair_mut(root.root.id());
    update_focus_chain_for_widget(
        &mut root.state,
        root_widget,
        root_state.reborrow_mut(),
        &mut dummy_focus_chain,
    );
}

// ----------------

// --- MARK: UPDATE FOCUS ---
pub(crate) fn run_update_focus_pass(root: &mut RenderRoot) {
    // If the focused widget is disabled, stashed or removed, we set
    // the focused id to None
    if let Some(id) = root.state.next_focused_widget {
        if !root.is_still_interactive(id) {
            root.state.next_focused_widget = None;
        }
    }

    let prev_focused = root.state.focused_widget;
    let next_focused = root.state.next_focused_widget;

    // "Focused path" means the focused widget, and all its parents.
    let prev_focused_path = std::mem::take(&mut root.state.focused_path);
    let next_focused_path = get_id_path(root, next_focused);

    let mut focused_set = HashSet::new();
    for widget_id in &next_focused_path {
        focused_set.insert(*widget_id);
    }

    trace!("prev_focused_path: {:?}", prev_focused_path);
    trace!("next_focused_path: {:?}", next_focused_path);

    // This is the same algorithm as the one in
    // run_update_pointer_pass
    // See comment in that function.

    fn update_focused_status_of(
        root: &mut RenderRoot,
        widget_id: WidgetId,
        focused_set: &HashSet<WidgetId>,
    ) {
        run_targeted_update_pass(root, Some(widget_id), |widget, ctx| {
            let has_focus = focused_set.contains(&ctx.widget_id());

            if ctx.widget_state.has_focus != has_focus {
                widget.on_status_change(ctx, &StatusChange::ChildFocusChanged(has_focus));
            }
            ctx.widget_state.has_focus = has_focus;
        });
    }

    // TODO - Make sure widgets are iterated from the bottom up.
    // TODO - Document the iteration order for update_focus pass.
    for widget_id in prev_focused_path.iter().copied() {
        if root.widget_arena.has(widget_id)
            && root.widget_arena.get_state_mut(widget_id).item.has_focus
                != focused_set.contains(&widget_id)
        {
            update_focused_status_of(root, widget_id, &focused_set);
        }
    }
    for widget_id in next_focused_path.iter().copied() {
        if root.widget_arena.has(widget_id)
            && root.widget_arena.get_state_mut(widget_id).item.has_focus
                != focused_set.contains(&widget_id)
        {
            update_focused_status_of(root, widget_id, &focused_set);
        }
    }

    if prev_focused != next_focused {
        let was_ime_active = root.state.is_ime_active;
        let is_ime_active = if let Some(id) = next_focused {
            root.widget_arena.get_state(id).item.accepts_text_input
        } else {
            false
        };
        root.state.is_ime_active = is_ime_active;

        run_single_update_pass(root, prev_focused, |widget, ctx| {
            widget.on_status_change(ctx, &StatusChange::FocusChanged(false));
        });
        run_single_update_pass(root, next_focused, |widget, ctx| {
            widget.on_status_change(ctx, &StatusChange::FocusChanged(true));
        });

        if prev_focused.is_some() && was_ime_active {
            root.state.emit_signal(RenderRootSignal::EndIme);
        }
        if next_focused.is_some() && is_ime_active {
            root.state.emit_signal(RenderRootSignal::StartIme);
        }

        if let Some(id) = next_focused {
            let ime_area = root.widget_arena.get_state(id).item.get_ime_area();
            root.state
                .emit_signal(RenderRootSignal::new_ime_moved_signal(ime_area));
        }
    }

    root.state.focused_widget = root.state.next_focused_widget;
    root.state.focused_path = next_focused_path;
}

// ----------------

// --- MARK: UPDATE SCROLL ---
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

// ----------------

// --- MARK: UPDATE POINTER ---
pub(crate) fn run_update_pointer_pass(root: &mut RenderRoot) {
    let pointer_pos = root.last_mouse_pos.map(|pos| (pos.x, pos.y).into());

    // Release pointer capture if target can no longer hold it.
    if let Some(id) = root.state.pointer_capture_target {
        if !root.is_still_interactive(id) {
            root.state.pointer_capture_target = None;
            run_on_pointer_event_pass(root, &PointerEvent::new_pointer_leave());
        }
    }

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
            let is_hovered = hovered_set.contains(&ctx.widget_id());

            if ctx.widget_state.is_hovered != is_hovered {
                widget.on_status_change(ctx, &StatusChange::HoveredChanged(is_hovered));
            }
            ctx.widget_state.is_hovered = is_hovered;
        });
    }

    // TODO - Make sure widgets are iterated from the bottom up.
    // TODO - Document the iteration order for update_pointer pass.
    for widget_id in prev_hovered_path.iter().copied() {
        if root.widget_arena.has(widget_id)
            && root.widget_arena.get_state_mut(widget_id).item.is_hovered
                != hovered_set.contains(&widget_id)
        {
            update_hovered_status_of(root, widget_id, &hovered_set);
        }
    }
    for widget_id in next_hovered_path.iter().copied() {
        if root.widget_arena.has(widget_id)
            && root.widget_arena.get_state_mut(widget_id).item.is_hovered
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
            .emit_signal(RenderRootSignal::SetCursor(new_cursor));
    }

    root.state.cursor_icon = new_cursor;
    root.state.hovered_path = next_hovered_path;
}
