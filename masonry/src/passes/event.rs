// Copyright 2024 the Xilem Authors
// SPDX-License-Identifier: Apache-2.0

use tracing::{debug, error, info_span, trace};

use crate::Handled;
use crate::app::{RenderRoot, RenderRootSignal};
use crate::core::{
    AccessEvent, EventCtx, PointerEvent, PointerInfo, PointerUpdate, PropertiesMut, TextEvent,
    Widget, WidgetId,
    keyboard::{Key, KeyState, NamedKey},
};
use crate::dpi::{LogicalPosition, PhysicalPosition};
use crate::passes::{enter_span, merge_state_up};

// --- MARK: HELPERS ---
fn get_pointer_target(
    root: &RenderRoot,
    pointer_pos: Option<LogicalPosition<f64>>,
) -> Option<WidgetId> {
    // See the [pointer capture documentation](../doc/06_masonry_concepts.md#pointer-capture).
    if let Some(capture_target) = root.global_state.pointer_capture_target {
        return Some(capture_target);
    }

    if let Some(pointer_pos) = pointer_pos {
        // TODO - Apply scale?
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
        PointerEvent::Down { state, .. }
        | PointerEvent::Up { state, .. }
        | PointerEvent::Move(PointerUpdate { current: state, .. })
        | PointerEvent::Scroll { state, .. } => Some(state.position),
        _ => None,
    }
}

fn run_event_pass<E>(
    root: &mut RenderRoot,
    target: Option<WidgetId>,
    event: &E,
    allow_pointer_capture: bool,
    pass_fn: impl FnMut(&mut dyn Widget, &mut EventCtx, &mut PropertiesMut<'_>, &E),
    trace: bool,
) -> Handled {
    let mut pass_fn = pass_fn;

    let original_target = target;
    let mut target_widget_id = target;
    let mut is_handled = false;
    while let Some(widget_id) = target_widget_id {
        if !root.widget_arena.has(widget_id) {
            error!(
                "Tried to access {widget_id} whilst processing event, but it wasn't in the tree. Discarding event"
            );
            break;
        }

        let parent_id = root.widget_arena.parent_of(widget_id);
        let (mut widget_mut, mut state_mut, mut properties_mut) =
            root.widget_arena.get_all_mut(widget_id);

        if !is_handled {
            let _span = enter_span(
                &root.global_state,
                &root.default_properties,
                widget_mut.reborrow(),
                state_mut.reborrow(),
                properties_mut.reborrow(),
            );
            let mut ctx = EventCtx {
                global_state: &mut root.global_state,
                widget_state: state_mut.item,
                widget_state_children: state_mut.children,
                widget_children: widget_mut.children,
                properties_children: properties_mut.children.reborrow_mut(),
                target: original_target.unwrap(),
                allow_pointer_capture,
                is_handled: false,
            };
            let widget = widget_mut.item;
            if trace {
                trace!(
                    "Widget '{}' {} visited",
                    widget.short_type_name(),
                    widget_id,
                );
            }

            let mut props = PropertiesMut {
                map: properties_mut.item,
                default_map: root.default_properties.for_widget(widget.type_id()),
            };
            pass_fn(&mut **widget, &mut ctx, &mut props, event);
            is_handled = ctx.is_handled;
        }

        merge_state_up(&mut root.widget_arena, widget_id);
        target_widget_id = parent_id;
    }

    Handled::from(is_handled)
}

// --- MARK: POINTER_EVENT ---
/// See the [passes documentation](../doc/05_pass_system.md#event-passes).
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
        root.global_state.needs_pointer_pass = true;
        root.last_mouse_pos = event_pos;
    }

    if root.global_state.inspector_state.is_picking_widget
        && matches!(event, PointerEvent::Move(..))
    {
        root.global_state.needs_pointer_pass = true;
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
        root.global_state.needs_pointer_pass = true;
        root.root_state_mut().needs_paint = true;
        return Handled::Yes;
    }

    let target_widget_id = get_pointer_target(root, event_pos);

    if matches!(event, PointerEvent::Down { .. }) {
        if let Some(target_widget_id) = target_widget_id {
            // The next tab event assign focus around this widget.
            root.global_state.most_recently_clicked_widget = Some(target_widget_id);

            // If we click outside of the focused widget, we clear the focus.
            if let Some(focused_widget) = root.global_state.focused_widget {
                // Focused_widget isn't ancestor of target_widget_id
                if !root
                    .widget_arena
                    .states
                    .get_id_path(target_widget_id)
                    .contains(&focused_widget.to_raw())
                {
                    root.global_state.next_focused_widget = None;
                }
            }
        }
    }

    let handled = run_event_pass(
        root,
        target_widget_id,
        event,
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

// --- MARK: TEXT EVENT ---
/// See the [passes documentation](../doc/05_pass_system.md#event-passes).
pub(crate) fn run_on_text_event_pass(root: &mut RenderRoot, event: &TextEvent) -> Handled {
    if matches!(event, TextEvent::WindowFocusChange(false)) {
        run_on_pointer_event_pass(
            root,
            &PointerEvent::Cancel(PointerInfo {
                pointer_id: None,
                persistent_device_id: None,
                pointer_type: Default::default(),
            }),
        );
    }

    let _span = info_span!("dispatch_text_event").entered();

    if event.is_high_density() {
        // We still want to record that this pass occurred in the debug file log.
        // However, we choose not record any other tracing for this event,
        // as that would have a lot of noise.
        trace!("Running ON_TEXT_EVENT pass with {}", event.short_name());
    } else {
        debug!("Running ON_TEXT_EVENT pass with {}", event.short_name());
    }

    if let TextEvent::WindowFocusChange(focused) = event {
        root.global_state.window_focused = *focused;
    }

    let target = root.global_state.focused_widget;

    let mut handled = run_event_pass(
        root,
        target,
        event,
        false,
        |widget, ctx, props, event| {
            widget.on_text_event(ctx, props, event);
        },
        !event.is_high_density(),
    );

    if let TextEvent::Keyboard(key) = event {
        // Handle Tab focus
        if key.key == Key::Named(NamedKey::Tab)
            && key.state == KeyState::Down
            && handled == Handled::No
        {
            let forward = !key.modifiers.shift();
            let next_focused_widget = root.widget_from_focus_chain(forward);
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
            root.global_state.needs_pointer_pass = true;
            root.root_state_mut().needs_paint = true;
            handled = Handled::Yes;
        }

        if key.key == Key::Named(NamedKey::F12)
            && key.state == KeyState::Down
            && handled == Handled::No
        {
            root.debug_paint = !root.debug_paint;
            root.root_state_mut().needs_paint = true;
            handled = Handled::Yes;
        }
    }

    if !event.is_high_density() {
        debug!(
            focused_widget = root.global_state.focused_widget.map(|id| id.0),
            handled = handled.is_handled(),
            "ON_TEXT_EVENT finished",
        );
    }

    handled
}

// --- MARK: ACCESS EVENT ---
/// See the [passes documentation](../doc/05_pass_system.md#event-passes).
pub(crate) fn run_on_access_event_pass(
    root: &mut RenderRoot,
    event: &AccessEvent,
    target: WidgetId,
) -> Handled {
    let _span = info_span!("access_event").entered();
    debug!("Running ON_ACCESS_EVENT pass with {}", event.short_name());

    let mut handled = run_event_pass(
        root,
        Some(target),
        event,
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
        _ => {}
    }

    debug!(
        focused_widget = root.global_state.focused_widget.map(|id| id.0),
        handled = handled.is_handled(),
        "ON_ACCESS_EVENT finished",
    );

    handled
}
