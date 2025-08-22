// Copyright 2024 the Xilem Authors
// SPDX-License-Identifier: Apache-2.0

use tracing::{debug, info_span, trace};
use ui_events::pointer::{PointerButtonEvent, PointerScrollEvent, PointerType};

use crate::app::{RenderRoot, RenderRootSignal};
use crate::core::keyboard::{Key, KeyState, NamedKey};
use crate::core::{
    AccessEvent, EventCtx, Handled, Ime, PointerEvent, PointerInfo, PointerUpdate, PropertiesMut,
    TextEvent, Widget, WidgetId,
};
use crate::debug_panic;
use crate::dpi::{LogicalPosition, PhysicalPosition};
use crate::passes::update::find_next_focusable;
use crate::passes::{enter_span, merge_state_up};

// --- MARK: HELPERS
fn get_pointer_target(
    root: &RenderRoot,
    pointer_pos: Option<LogicalPosition<f64>>,
) -> Option<WidgetId> {
    // See the [pointer capture documentation](../doc/06_masonry_concepts.md#pointer-capture).
    if let Some(capture_target) = root.global_state.pointer_capture_target
        && root.widget_arena.has(capture_target)
    {
        return Some(capture_target);
    }

    if let Some(pointer_pos) = pointer_pos {
        // TODO - Apply scale
        let pointer_pos = (pointer_pos.x, pointer_pos.y).into();
        return root
            .get_root_widget()
            .find_widget_under_pointer(pointer_pos)
            .map(|widget| widget.id());
    }

    None
}

/// `true` if this [`PointerEvent`] type is likely to occur every frame.
fn is_very_frequent(e: &PointerEvent) -> bool {
    matches!(e, PointerEvent::Move(..) | PointerEvent::Scroll { .. })
}

/// Short name for a [`PointerEvent`].
fn pointer_event_short_name(e: &PointerEvent) -> &'static str {
    match e {
        PointerEvent::Down { .. } => "Down",
        PointerEvent::Up { .. } => "Up",
        PointerEvent::Move(..) => "Move",
        PointerEvent::Enter(..) => "Enter",
        PointerEvent::Leave(..) => "Leave",
        PointerEvent::Cancel(..) => "Cancel",
        PointerEvent::Scroll { .. } => "Scroll",
    }
}

/// A position if the event has one.
fn try_event_position(event: &PointerEvent) -> Option<PhysicalPosition<f64>> {
    match event {
        PointerEvent::Down(PointerButtonEvent { state, .. })
        | PointerEvent::Up(PointerButtonEvent { state, .. })
        | PointerEvent::Move(PointerUpdate { current: state, .. })
        | PointerEvent::Scroll(PointerScrollEvent { state, .. }) => Some(state.position),
        _ => None,
    }
}

fn run_event_pass<E>(
    root: &mut RenderRoot,
    target: Option<WidgetId>,
    event: &E,
    skip_if_disabled: bool,
    allow_pointer_capture: bool,
    pass_fn: impl FnMut(&mut dyn Widget, &mut EventCtx<'_>, &mut PropertiesMut<'_>, &E),
    trace: bool,
) -> Handled {
    let mut pass_fn = pass_fn;

    if let Some(id) = target
        && !root.has_widget(id)
    {
        debug_panic!("Cannot send event to non-existent widget {id}.");
        return Handled::No;
    }

    if let Some(id) = target {
        let state = root.widget_arena.get_state(id);

        if state.is_disabled && skip_if_disabled {
            return Handled::No;
        }
    }

    let original_target = target;
    let mut target_widget_id = target;
    let mut is_handled = false;
    while let Some(widget_id) = target_widget_id {
        let parent_id = root.widget_arena.parent_of(widget_id);
        let mut node = root.widget_arena.get_node_mut(widget_id);

        if !is_handled {
            let _span = enter_span(&node.item.state);
            let mut ctx = EventCtx {
                global_state: &mut root.global_state,
                widget_state: &mut node.item.state,
                children: node.children.reborrow_mut(),
                default_properties: &root.default_properties,
                target: original_target.unwrap(),
                allow_pointer_capture,
                is_handled: false,
            };
            let widget = &mut *node.item.widget;
            if trace {
                trace!(
                    "Widget '{}' {} visited",
                    widget.short_type_name(),
                    widget_id,
                );
            }

            let mut props = PropertiesMut {
                map: &mut node.item.properties,
                default_map: root.default_properties.for_widget(widget.type_id()),
            };
            pass_fn(widget, &mut ctx, &mut props, event);
            is_handled = ctx.is_handled;
        }

        merge_state_up(&mut root.widget_arena, widget_id);
        target_widget_id = parent_id;
    }

    Handled::from(is_handled)
}

// --- MARK: POINTER_EVENT
/// See the [passes documentation](crate::doc::pass_system#event-passes).
pub(crate) fn run_on_pointer_event_pass(root: &mut RenderRoot, event: &PointerEvent) -> Handled {
    let _span = info_span!("dispatch_pointer_event").entered();

    if is_very_frequent(event) {
        // We still want to record that this pass occurred in the debug file log.
        // However, we choose not to record any other tracing for this event,
        // as that would create a lot of noise.
        trace!(
            "Running ON_POINTER_EVENT pass with {}",
            pointer_event_short_name(event)
        );
    } else {
        debug!(
            "Running ON_POINTER_EVENT pass with {}",
            pointer_event_short_name(event)
        );
    }

    let event_pos = try_event_position(event).map(|p| p.to_logical(root.global_state.scale_factor));

    if event_pos != root.last_mouse_pos {
        root.last_mouse_pos = event_pos;
    }
    root.global_state.needs_pointer_pass = true;

    if root.global_state.inspector_state.is_picking_widget
        && matches!(event, PointerEvent::Move(..))
    {
        return Handled::Yes;
    }

    // If the widget picker is active and this is a click event,
    // we select the widget under the mouse and short-circuit the event pass.
    if root.global_state.inspector_state.is_picking_widget
        && matches!(event, PointerEvent::Down { .. })
    {
        let target_widget_id = get_pointer_target(root, event_pos);
        if let Some(target_widget_id) = target_widget_id {
            root.global_state
                .emit_signal(RenderRootSignal::WidgetSelectedInInspector(
                    target_widget_id,
                ));
        }
        root.global_state.inspector_state.is_picking_widget = false;
        root.global_state.inspector_state.hovered_widget = None;
        root.root_state_mut().needs_paint = true;
        return Handled::Yes;
    }

    let target_widget_id = get_pointer_target(root, event_pos);

    if matches!(event, PointerEvent::Down { .. })
        && let Some(target_widget_id) = target_widget_id
    {
        // The next tab event assign focus around this widget.
        root.global_state.focus_anchor = Some(target_widget_id);

        // If we click outside of the focused widget, we clear the focus.
        if let Some(focused_widget) = root.global_state.focused_widget {
            // Focused_widget isn't ancestor of target_widget_id
            if !root
                .widget_arena
                .nodes
                .get_id_path(target_widget_id)
                .contains(&focused_widget.to_raw())
            {
                root.global_state.next_focused_widget = None;
            }
        }
    }

    let skip_if_disabled = !matches!(event, PointerEvent::Cancel { .. });
    let handled = run_event_pass(
        root,
        target_widget_id,
        event,
        skip_if_disabled,
        matches!(event, PointerEvent::Down { .. }),
        |widget, ctx, props, event| {
            widget.on_pointer_event(ctx, props, event);
        },
        !is_very_frequent(event),
    );

    if matches!(event, PointerEvent::Up { .. } | PointerEvent::Cancel(..)) {
        // Automatically release the pointer on pointer up or leave. If a widget holds the capture,
        // it is notified of the pointer event before the capture is released, so it knows it is
        // about to lose the pointer.
        root.global_state.pointer_capture_target = None;
    }

    if !is_very_frequent(event) {
        debug!(
            focused_widget = root.global_state.focused_widget.map(|id| id.0),
            handled = handled.is_handled(),
            "ON_POINTER_EVENT finished",
        );
    }

    handled
}

// --- MARK: TEXT EVENT
/// See the [passes documentation](crate::doc::pass_system#event-passes).
pub(crate) fn run_on_text_event_pass(root: &mut RenderRoot, event: &TextEvent) -> Handled {
    if matches!(event, TextEvent::WindowFocusChange(false)) {
        run_on_pointer_event_pass(
            root,
            &PointerEvent::Cancel(PointerInfo {
                pointer_id: None,
                persistent_device_id: None,
                pointer_type: PointerType::default(),
            }),
        );
    }

    let _span = info_span!("dispatch_text_event").entered();

    debug!("Running ON_TEXT_EVENT pass with {}", event.short_name());

    if let TextEvent::WindowFocusChange(focused) = event {
        root.global_state.window_focused = *focused;
    }

    let target = root.global_state.focused_widget.or_else(|| {
        if let Some(focus_fallback) = root.global_state.focus_fallback
            && root.is_still_interactive(focus_fallback)
        {
            Some(focus_fallback)
        } else {
            None
        }
    });

    let skip_if_disabled = !matches!(event, TextEvent::Ime(Ime::Disabled));
    let mut handled = run_event_pass(
        root,
        target,
        event,
        skip_if_disabled,
        false,
        |widget, ctx, props, event| {
            widget.on_text_event(ctx, props, event);
        },
        true,
    );

    if let TextEvent::Keyboard(key) = event {
        // Handle Tab focus
        if key.key == Key::Named(NamedKey::Tab)
            && key.state == KeyState::Down
            && handled == Handled::No
        {
            let forward = !key.modifiers.shift();
            let next_focused_widget = find_next_focusable(root, forward);
            root.global_state.next_focused_widget = next_focused_widget;
            handled = Handled::Yes;
        }

        if key.key == Key::Named(NamedKey::F11)
            && key.state == KeyState::Down
            && handled == Handled::No
        {
            root.global_state.inspector_state.is_picking_widget =
                !root.global_state.inspector_state.is_picking_widget;
            root.global_state.inspector_state.hovered_widget = None;
            root.root_state_mut().needs_paint = true;
            handled = Handled::Yes;
        }

        if key.key == Key::Named(NamedKey::F12)
            && key.state == KeyState::Down
            && handled == Handled::No
        {
            root.global_state.debug_paint = !root.global_state.debug_paint;
            root.root_state_mut().needs_paint = true;
            handled = Handled::Yes;
        }
    }

    debug!(
        focused_widget = root.global_state.focused_widget.map(|id| id.0),
        handled = handled.is_handled(),
        "ON_TEXT_EVENT finished",
    );

    handled
}

// --- MARK: ACCESS EVENT
/// See the [passes documentation](crate::doc::pass_system#event-passes).
pub(crate) fn run_on_access_event_pass(
    root: &mut RenderRoot,
    event: &AccessEvent,
    target: WidgetId,
) -> Handled {
    let _span = info_span!("access_event").entered();
    debug!("Running ON_ACCESS_EVENT pass with {}", event.short_name());

    let skip_if_disabled = true;
    let mut handled = run_event_pass(
        root,
        Some(target),
        event,
        skip_if_disabled,
        false,
        |widget, ctx, props, event| {
            widget.on_access_event(ctx, props, event);
        },
        true,
    );

    // Handle focus events
    match event.action {
        accesskit::Action::Focus if !handled.is_handled() => {
            if root.is_still_interactive(target) {
                root.global_state.next_focused_widget = Some(target);
                handled = Handled::Yes;
            }
        }
        accesskit::Action::Blur if !handled.is_handled() => {
            if root.global_state.next_focused_widget == Some(target) {
                root.global_state.next_focused_widget = None;
                handled = Handled::Yes;
            }
        }
        accesskit::Action::ScrollIntoView if !handled.is_handled() => {
            let widget_state = root.widget_arena.get_state(target);
            let rect = widget_state.layout_rect();
            root.global_state
                .scroll_request_targets
                .push((target, rect));
            handled = Handled::Yes;
        }
        _ => {}
    }

    debug!(
        focused_widget = root.global_state.focused_widget.map(|id| id.0),
        handled = handled.is_handled(),
        "ON_ACCESS_EVENT finished",
    );

    handled
}
