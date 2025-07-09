// Copyright 2024 the Xilem Authors
// SPDX-License-Identifier: Apache-2.0

use std::collections::HashSet;

use cursor_icon::CursorIcon;
use tracing::{info_span, trace};
use tree_arena::ArenaMut;
use ui_events::pointer::PointerType;

use crate::app::{RenderRoot, RenderRootSignal, RenderRootState};
use crate::core::{
    DefaultProperties, Ime, PointerEvent, PointerInfo, PropertiesMut, PropertiesRef, QueryCtx,
    RegisterCtx, TextEvent, Update, UpdateCtx, Widget, WidgetId, WidgetState,
};
use crate::passes::event::{run_on_pointer_event_pass, run_on_text_event_pass};
use crate::passes::{enter_span, enter_span_if, merge_state_up, recurse_on_children};
use crate::util::AnyMap;

// --- MARK: HELPERS
/// Returns the id path starting from the given widget id and ending at the root.
///
/// If `widget_id` is `None`, returns an empty `Vec`.
fn get_id_path(root: &RenderRoot, widget_id: Option<WidgetId>) -> Vec<WidgetId> {
    let Some(widget_id) = widget_id else {
        return Vec::new();
    };

    root.widget_arena
        .states
        .get_id_path(widget_id)
        .iter()
        .map(|&id| WidgetId(id.try_into().unwrap()))
        .collect()
}

/// Make a dummy [`PointerEvent::Cancel`].
fn dummy_pointer_cancel() -> PointerEvent {
    PointerEvent::Cancel(PointerInfo {
        pointer_id: None,
        persistent_device_id: None,
        pointer_type: PointerType::default(),
    })
}

fn run_targeted_update_pass(
    root: &mut RenderRoot,
    target: Option<WidgetId>,
    mut pass_fn: impl FnMut(&mut dyn Widget, &mut UpdateCtx<'_>, &mut PropertiesMut<'_>),
) {
    let mut current_id = target;
    while let Some(widget_id) = current_id {
        let parent_id = root.widget_arena.parent_of(widget_id);
        let (widget_mut, state_mut, properties_mut) = root.widget_arena.get_all_mut(widget_id);

        let widget = &mut **widget_mut.item;
        let mut ctx = UpdateCtx {
            global_state: &mut root.global_state,
            widget_state: state_mut.item,
            widget_state_children: state_mut.children,
            widget_children: widget_mut.children,
            properties_children: properties_mut.children,
        };
        let mut props = PropertiesMut {
            map: properties_mut.item,
            default_map: root.default_properties.for_widget(widget.type_id()),
        };
        pass_fn(widget, &mut ctx, &mut props);

        merge_state_up(&mut root.widget_arena, widget_id);
        current_id = parent_id;
    }
}

fn run_single_update_pass(
    root: &mut RenderRoot,
    target: Option<WidgetId>,
    mut pass_fn: impl FnMut(&mut dyn Widget, &mut UpdateCtx<'_>, &mut PropertiesMut<'_>),
) {
    let Some(target) = target else {
        return;
    };
    if !root.widget_arena.has(target) {
        return;
    }

    let (widget_mut, state_mut, properties_mut) = root.widget_arena.get_all_mut(target);

    let mut ctx = UpdateCtx {
        global_state: &mut root.global_state,
        widget_state: state_mut.item,
        widget_state_children: state_mut.children,
        widget_children: widget_mut.children,
        properties_children: properties_mut.children,
    };
    let mut props = PropertiesMut {
        map: properties_mut.item,
        default_map: root
            .default_properties
            .for_widget(widget_mut.item.type_id()),
    };
    pass_fn(&mut **widget_mut.item, &mut ctx, &mut props);

    let mut current_id = Some(target);
    while let Some(widget_id) = current_id {
        merge_state_up(&mut root.widget_arena, widget_id);
        current_id = root.widget_arena.parent_of(widget_id);
    }
}

// --- MARK: TREE
fn update_widget_tree(
    global_state: &mut RenderRootState,
    default_properties: &DefaultProperties,
    mut widget: ArenaMut<'_, Box<dyn Widget>>,
    mut state: ArenaMut<'_, WidgetState>,
    mut properties: ArenaMut<'_, AnyMap>,
) {
    let trace = global_state.trace.update_tree;
    let _span = enter_span_if(
        trace,
        global_state,
        default_properties,
        widget.reborrow(),
        state.reborrow(),
        properties.reborrow(),
    );
    let id = state.item.id;

    if !state.item.children_changed {
        return;
    }
    state.item.children_changed = false;

    {
        let mut ctx = RegisterCtx {
            widget_state_children: state.children.reborrow_mut(),
            widget_children: widget.children.reborrow_mut(),
            properties_children: properties.children.reborrow_mut(),
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
                        "Error in '{}' {}: method register_children() called \
                        RegisterCtx::register_child() on child {}, which isn't \
                        in the list returned by children_ids()",
                        widget.item.short_type_name(),
                        id,
                        child_id
                    );
                }
            }
        }

        #[cfg(debug_assertions)]
        for child_id in widget.item.children_ids() {
            if widget.children.item(child_id).is_none() {
                panic!(
                    "Error in '{}' {}: method register_children() did not call \
                    RegisterCtx::register_child() on child {} returned by children_ids()",
                    widget.item.short_type_name(),
                    id,
                    child_id
                );
            }
        }
    }

    if state.item.is_new {
        let mut ctx = UpdateCtx {
            global_state,
            widget_state: state.item,
            widget_state_children: state.children.reborrow_mut(),
            widget_children: widget.children.reborrow_mut(),
            properties_children: properties.children.reborrow_mut(),
        };
        let mut props = PropertiesMut {
            map: properties.item,
            default_map: default_properties.for_widget(widget.item.type_id()),
        };
        widget
            .item
            .update(&mut ctx, &mut props, &Update::WidgetAdded);
        if trace {
            trace!(
                "{} received Update::WidgetAdded",
                widget.item.short_type_name()
            );
        }
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
        properties.children,
        |widget, mut state, properties| {
            update_widget_tree(
                global_state,
                default_properties,
                widget,
                state.reborrow_mut(),
                properties,
            );
            parent_state.merge_up(state.item);
        },
    );
}

/// See the [passes documentation](../doc/05_pass_system.md#update-tree-pass).
pub(crate) fn run_update_widget_tree_pass(root: &mut RenderRoot) {
    let _span = info_span!("update_new_widgets").entered();

    if root.root.incomplete() {
        let mut ctx = RegisterCtx {
            widget_state_children: root.widget_arena.states.roots_mut(),
            widget_children: root.widget_arena.widgets.roots_mut(),
            properties_children: root.widget_arena.properties.roots_mut(),
            #[cfg(debug_assertions)]
            registered_ids: Vec::new(),
        };
        ctx.register_child(&mut root.root);
    }

    let (root_widget, mut root_state, root_properties) =
        root.widget_arena.get_all_mut(root.root.id());
    update_widget_tree(
        &mut root.global_state,
        &root.default_properties,
        root_widget,
        root_state.reborrow_mut(),
        root_properties,
    );
}

// ----------------

// --- MARK: UPDATE DISABLED
/// See the [passes documentation](../doc/05_pass_system.md#update-passes).
/// See the [disabled status documentation](../doc/06_masonry_concepts.md#disabled).
fn update_disabled_for_widget(
    global_state: &mut RenderRootState,
    default_properties: &DefaultProperties,
    mut widget: ArenaMut<'_, Box<dyn Widget>>,
    mut state: ArenaMut<'_, WidgetState>,
    mut properties: ArenaMut<'_, AnyMap>,
    parent_disabled: bool,
) {
    let _span = enter_span(
        global_state,
        default_properties,
        widget.reborrow(),
        state.reborrow(),
        properties.reborrow(),
    );
    let id = state.item.id;

    let disabled = state.item.is_explicitly_disabled || parent_disabled;
    if !state.item.needs_update_disabled && disabled == state.item.is_disabled {
        return;
    }

    if disabled != state.item.is_disabled {
        let mut ctx = UpdateCtx {
            global_state,
            widget_state: state.item,
            widget_state_children: state.children.reborrow_mut(),
            widget_children: widget.children.reborrow_mut(),
            properties_children: properties.children.reborrow_mut(),
        };
        let mut props = PropertiesMut {
            map: properties.item,
            default_map: default_properties.for_widget(widget.item.type_id()),
        };
        widget
            .item
            .update(&mut ctx, &mut props, &Update::DisabledChanged(disabled));
        state.item.is_disabled = disabled;
        state.item.needs_update_focus_chain = true;
        state.item.request_accessibility = true;
        state.item.needs_accessibility = true;
    }

    state.item.needs_update_disabled = false;

    let parent_state = state.item;
    recurse_on_children(
        id,
        widget.reborrow_mut(),
        state.children,
        properties.children,
        |widget, mut state, properties| {
            update_disabled_for_widget(
                global_state,
                default_properties,
                widget,
                state.reborrow_mut(),
                properties,
                disabled,
            );
            parent_state.merge_up(state.item);
        },
    );
}

pub(crate) fn run_update_disabled_pass(root: &mut RenderRoot) {
    let _span = info_span!("update_disabled").entered();

    // If a widget was enabled or disabled, the pointer icon may need to change.
    if root.root_state().needs_update_disabled {
        root.global_state.needs_pointer_pass = true;
    }

    let (root_widget, root_state, root_properties) = root.widget_arena.get_all_mut(root.root.id());
    update_disabled_for_widget(
        &mut root.global_state,
        &root.default_properties,
        root_widget,
        root_state,
        root_properties,
        false,
    );
}

// ----------------

// *Stashed* is for widgets that are no longer "part of the graph". So they can't get keyboard events, don't get painted, etc, but should keep some state.
// Scrolled-out widgets are *not* stashed.

// --- MARK: UPDATE STASHED
/// See the [passes documentation](../doc/05_pass_system.md#update-passes).
/// See the [stashed status documentation](../doc/06_masonry_concepts.md#stashed).
fn update_stashed_for_widget(
    global_state: &mut RenderRootState,
    default_properties: &DefaultProperties,
    mut widget: ArenaMut<'_, Box<dyn Widget>>,
    mut state: ArenaMut<'_, WidgetState>,
    mut properties: ArenaMut<'_, AnyMap>,
    parent_stashed: bool,
) {
    let _span = enter_span(
        global_state,
        default_properties,
        widget.reborrow(),
        state.reborrow(),
        properties.reborrow(),
    );
    let id = state.item.id;

    let stashed = state.item.is_explicitly_stashed || parent_stashed;
    if !state.item.needs_update_stashed && stashed == state.item.is_stashed {
        return;
    }

    if stashed != state.item.is_stashed {
        let mut ctx = UpdateCtx {
            global_state,
            widget_state: state.item,
            widget_state_children: state.children.reborrow_mut(),
            widget_children: widget.children.reborrow_mut(),
            properties_children: properties.children.reborrow_mut(),
        };
        let mut props = PropertiesMut {
            map: properties.item,
            default_map: default_properties.for_widget(widget.item.type_id()),
        };
        widget
            .item
            .update(&mut ctx, &mut props, &Update::StashedChanged(stashed));
        state.item.is_stashed = stashed;
        state.item.needs_update_focus_chain = true;
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
        properties.children,
        |widget, mut state, properties| {
            update_stashed_for_widget(
                global_state,
                default_properties,
                widget,
                state.reborrow_mut(),
                properties,
                stashed,
            );
            parent_state.merge_up(state.item);
        },
    );
}

pub(crate) fn run_update_stashed_pass(root: &mut RenderRoot) {
    let _span = info_span!("update_stashed").entered();

    let (root_widget, root_state, root_properties) = root.widget_arena.get_all_mut(root.root.id());
    update_stashed_for_widget(
        &mut root.global_state,
        &root.default_properties,
        root_widget,
        root_state,
        root_properties,
        false,
    );
}

// ----------------

// --- MARK: FOCUS CHAIN

// TODO https://github.com/linebender/xilem/issues/376 - Some implicit invariants:
// - A widget only receives BuildFocusChain if none of its parents are hidden.

// TODO - This logic was copy-pasted from WidgetPod code and may need to be refactored.
// It doesn't quite behave like other update passes (for instance, some code runs after
// recurse_on_children), and some design decisions inherited from Druid should be reconsidered.
/// See the [passes documentation](../doc/05_pass_system.md#update-passes).
fn update_focus_chain_for_widget(
    global_state: &mut RenderRootState,
    default_properties: &DefaultProperties,
    mut widget: ArenaMut<'_, Box<dyn Widget>>,
    mut state: ArenaMut<'_, WidgetState>,
    mut properties: ArenaMut<'_, AnyMap>,
    parent_focus_chain: &mut Vec<WidgetId>,
) {
    let _span = enter_span(
        global_state,
        default_properties,
        widget.reborrow(),
        state.reborrow(),
        properties.reborrow(),
    );
    let id = state.item.id;

    // Replace has_focused to check if the value changed in the meantime
    state.item.has_focus_target = global_state.focused_widget == Some(id);
    let had_focus = state.item.has_focus_target;

    if state.item.needs_update_focus_chain {
        state.item.focus_chain.clear();
        if state.item.accepts_focus {
            state.item.focus_chain.push(id);
        }
        state.item.needs_update_focus_chain = false;

        let parent_state = &mut *state.item;
        recurse_on_children(
            id,
            widget.reborrow_mut(),
            state.children,
            properties.children,
            |widget, mut state, properties| {
                update_focus_chain_for_widget(
                    global_state,
                    default_properties,
                    widget,
                    state.reborrow_mut(),
                    properties,
                    &mut parent_state.focus_chain,
                );
                parent_state.merge_up(state.item);
            },
        );
    }

    if !state.item.is_disabled {
        parent_focus_chain.extend(&state.item.focus_chain);
    }

    // had_focus is the old focus value. state.has_focused was replaced with parent_ctx.is_focused().
    // Therefore if had_focus is true but state.has_focused is false then the widget which is
    // currently focused is not part of the functional tree anymore and should resign the focus.
    if had_focus && !state.item.has_focus_target {
        // Not sure about this logic, might remove
        global_state.next_focused_widget = None;
    }
    state.item.has_focus_target = had_focus;
}

pub(crate) fn run_update_focus_chain_pass(root: &mut RenderRoot) {
    let _span = info_span!("update_focus_chain").entered();
    let mut dummy_focus_chain = Vec::new();

    let (root_widget, root_state, root_properties) = root.widget_arena.get_all_mut(root.root.id());
    update_focus_chain_for_widget(
        &mut root.global_state,
        &root.default_properties,
        root_widget,
        root_state,
        root_properties,
        &mut dummy_focus_chain,
    );
}

// ----------------

// --- MARK: UPDATE FOCUS
/// See the [passes documentation](../doc/05_pass_system.md#update-passes).
/// See the [focus status documentation](../doc/06_masonry_concepts.md#text-focus).
pub(crate) fn run_update_focus_pass(root: &mut RenderRoot) {
    let _span = info_span!("update_focus").entered();
    // If the next-focused widget is disabled, stashed or removed, we set
    // the focused id to None
    if let Some(id) = root.global_state.next_focused_widget {
        if !root.is_still_interactive(id) {
            root.global_state.next_focused_widget = None;
        }
    }

    let prev_focused = root.global_state.focused_widget;
    let was_ime_active = root.global_state.is_ime_active;

    if was_ime_active && prev_focused != root.global_state.next_focused_widget {
        // IME was active, but the next focused widget is going to receive the Ime::Disabled event
        // sent by the platform. Synthesize an `Ime::Disabled` event here and send it to the widget
        // about to be unfocused.

        // HACK: It's not valid to send an event to a non-existent widget, so we check that the "previously"
        // focused widget hasn't just been deleted.
        // This means that if a parent widget was handling IME events, it won't get this event.
        // We know that IME events bubbling is not the correct behaviour, but have chosen to keep it for consistency,
        // as we also are planning to refactor how IME is delivered as we update to use Android View.
        if let Some(prev_focused) = prev_focused
            && root.has_widget(prev_focused)
        {
            run_on_text_event_pass(root, &TextEvent::Ime(Ime::Disabled));
        }

        // Disable the IME, which was enabled specifically for this widget. Note that if the newly
        // focused widget also requires IME, we will request it again - this resets the platform's
        // state, ensuring that partial IME inputs do not "travel" between widgets
        root.global_state.emit_signal(RenderRootSignal::EndIme);

        // Note: handling of the Ime::Disabled event sent above may have changed the next focused
        // widget. In particular, focus may have changed back to the original widget we just
        // disabled IME for.
        //
        // In this unlikely case, the rest of this handler will short-circuit, and IME would not be
        // re-enabled for this widget. Re-enable IME here; the resultant `Ime::Enabled` event sent
        // by the platform will be routed to this widget as it remains the focused widget. We don't
        // handle this as above to avoid loops.
        //
        // First do the disabled, stashed or removed check again.
        if let Some(id) = root.global_state.next_focused_widget {
            if !root.is_still_interactive(id) {
                root.global_state.next_focused_widget = None;
            }
        }
        if prev_focused == root.global_state.next_focused_widget {
            tracing::warn!(
                id = prev_focused.map(|id| id.trace()),
                "request_focus called whilst handling Ime::Disabled"
            );
            root.global_state.emit_signal(RenderRootSignal::StartIme);
        }
    }

    let next_focused = root.global_state.next_focused_widget;

    // "Focused path" means the focused widget, and all its parents.
    let prev_focused_path = std::mem::take(&mut root.global_state.focused_path);
    let next_focused_path = get_id_path(root, next_focused);

    // We don't just compare `prev_focused` and `next_focused` they could be the same widget
    // but one of their ancestors could have been reparented.
    // (assuming we ever implement reparenting)
    if prev_focused_path != next_focused_path {
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
            run_targeted_update_pass(root, Some(widget_id), |widget, ctx, props| {
                let has_focused = focused_set.contains(&ctx.widget_id());

                if ctx.widget_state.has_focus_target != has_focused {
                    widget.update(ctx, props, &Update::ChildFocusChanged(has_focused));
                }
                ctx.widget_state.has_focus_target = has_focused;
            });
        }

        // TODO - Add unit test to check items are iterated from the bottom up.
        for widget_id in prev_focused_path.iter().copied() {
            if root.widget_arena.has(widget_id)
                && root
                    .widget_arena
                    .get_state_mut(widget_id)
                    .item
                    .has_focus_target
                    != focused_set.contains(&widget_id)
            {
                update_focused_status_of(root, widget_id, &focused_set);
            }
        }
        for widget_id in next_focused_path.iter().copied() {
            if root.widget_arena.has(widget_id)
                && root
                    .widget_arena
                    .get_state_mut(widget_id)
                    .item
                    .has_focus_target
                    != focused_set.contains(&widget_id)
            {
                update_focused_status_of(root, widget_id, &focused_set);
            }
        }
    }

    // Refocus if the focused widget changed.
    if prev_focused != next_focused {
        // We send FocusChange event to widget that lost and the widget that gained focus.
        // We also request accessibility, because build_access_node() depends on the focus state.
        run_single_update_pass(root, prev_focused, |widget, ctx, props| {
            widget.update(ctx, props, &Update::FocusChanged(false));
            ctx.widget_state.request_accessibility = true;
            ctx.widget_state.needs_accessibility = true;
        });
        run_single_update_pass(root, next_focused, |widget, ctx, props| {
            widget.update(ctx, props, &Update::FocusChanged(true));
            ctx.widget_state.request_accessibility = true;
            ctx.widget_state.needs_accessibility = true;
        });

        if let Some(next_focused) = next_focused {
            let widget_state = root.widget_arena.get_state(next_focused).item;

            root.global_state.is_ime_active = widget_state.accepts_text_input;
            if widget_state.accepts_text_input {
                root.global_state.emit_signal(RenderRootSignal::StartIme);
            }
        } else {
            root.global_state.is_ime_active = false;
        }
    }

    root.global_state.focused_widget = next_focused;
    root.global_state.focused_path = next_focused_path;
}

// ----------------

// --- MARK: SCROLL
// This pass will update scroll positions in cases where a widget has requested to be
// scrolled into view (usually a text input getting text events).
// Each parent that implements scrolling will update its scroll position to ensure the
// child is visible. (If the target area is larger than the parent, the parent will try
// to show the top left of that area.)
/// See the [passes documentation](../doc/05_pass_system.md#update-passes).
pub(crate) fn run_update_scroll_pass(root: &mut RenderRoot) {
    let _span = info_span!("update_scroll").entered();

    let scroll_request_targets = std::mem::take(&mut root.global_state.scroll_request_targets);
    for (target, rect) in scroll_request_targets {
        let mut target_rect = rect;

        // TODO - Run top-down instead of bottom-up.
        run_targeted_update_pass(root, Some(target), |widget, ctx, props| {
            let event = Update::RequestPanToChild(rect);
            widget.update(ctx, props, &event);

            // TODO - We should run the compose method after this, so
            // translations are updated and the rect passed to parents
            // is more accurate.

            let state = &ctx.widget_state;
            target_rect = target_rect + state.scroll_translation + state.origin.to_vec2();
        });
    }
}

// ----------------

// --- MARK: UPDATE POINTER
/// See the [passes documentation](../doc/05_pass_system.md#update-passes).
pub(crate) fn run_update_pointer_pass(root: &mut RenderRoot) {
    if !root.global_state.needs_pointer_pass {
        return;
    }
    let _span = info_span!("update_pointer").entered();

    root.global_state.needs_pointer_pass = false;

    let pointer_pos = root.last_mouse_pos.map(|pos| (pos.x, pos.y).into());

    if root.global_state.inspector_state.is_picking_widget {
        if let Some(pos) = pointer_pos {
            root.global_state.inspector_state.hovered_widget = root
                .get_root_widget()
                .find_widget_under_pointer(pos)
                .map(|widget| widget.id());
        }
        root.root_state_mut().needs_paint = true;
        return;
    }

    // Release pointer capture if target can no longer hold it.
    if let Some(id) = root.global_state.pointer_capture_target {
        if !root.is_still_interactive(id) {
            // The event pass will set pointer_capture_target to None.
            run_on_pointer_event_pass(root, &dummy_pointer_cancel());
        }
    }

    // -- UPDATE HOVERED WIDGETS --
    let mut next_hovered_widget = if let Some(pos) = pointer_pos {
        // TODO - Apply scale?
        root.get_root_widget()
            .find_widget_under_pointer(pos)
            .map(|widget| widget.id())
    } else {
        None
    };
    // If the pointer is captured, it can either hover its capture target or nothing.
    if let Some(capture_target) = root.global_state.pointer_capture_target {
        if next_hovered_widget != Some(capture_target) {
            next_hovered_widget = None;
        }
    }

    // "Hovered path" means the widget which is considered hovered, and all its parents.
    let prev_hovered_path = std::mem::take(&mut root.global_state.hovered_path);
    let next_hovered_path = get_id_path(root, next_hovered_widget);
    let prev_hovered_widget = prev_hovered_path.first().copied();

    // We don't just compare `prev_focused` and `next_focused` they could be the same widget
    // but one of their ancestors could have been reparented.
    // (assuming we ever implement reparenting)
    if prev_hovered_path != next_hovered_path {
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
            run_targeted_update_pass(root, Some(widget_id), |widget, ctx, props| {
                let has_hovered = hovered_set.contains(&ctx.widget_id());

                if ctx.widget_state.has_hovered != has_hovered {
                    widget.update(ctx, props, &Update::ChildHoveredChanged(has_hovered));
                }
                ctx.widget_state.has_hovered = has_hovered;
            });
        }

        // TODO - Add unit test to check items are iterated from the bottom up.
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
    }

    if prev_hovered_widget != next_hovered_widget {
        run_single_update_pass(root, prev_hovered_widget, |widget, ctx, props| {
            ctx.widget_state.is_hovered = false;
            widget.update(ctx, props, &Update::HoveredChanged(false));
        });
        run_single_update_pass(root, next_hovered_widget, |widget, ctx, props| {
            ctx.widget_state.is_hovered = true;
            widget.update(ctx, props, &Update::HoveredChanged(true));
        });
    }

    // -- UPDATE CURSOR ICON --

    // If the pointer is captured, its icon always reflects the
    // capture target, even when not hovered.
    let icon_source = root
        .global_state
        .pointer_capture_target
        .or(next_hovered_widget);

    let new_icon = if let (Some(icon_source), Some(pos)) = (icon_source, pointer_pos) {
        let (widget, state, properties) = root.widget_arena.get_all(icon_source);

        let ctx = QueryCtx {
            global_state: &root.global_state,
            widget_state_children: state.children,
            widget_children: widget.children,
            widget_state: state.item,
            properties: PropertiesRef {
                map: properties.item,
                default_map: root.default_properties.for_widget(widget.item.type_id()),
            },
            properties_children: properties.children,
        };

        if state.item.is_disabled {
            CursorIcon::Default
        } else {
            widget.item.get_cursor(&ctx, pos)
        }
    } else {
        CursorIcon::Default
    };

    if root.global_state.cursor_icon != new_icon {
        root.global_state
            .emit_signal(RenderRootSignal::SetCursor(new_icon));
    }

    root.global_state.cursor_icon = new_icon;
    root.global_state.hovered_path = next_hovered_path;
}
