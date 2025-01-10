// Copyright 2024 the Xilem Authors
// SPDX-License-Identifier: Apache-2.0

use dpi::LogicalPosition;
use tracing::{debug, info_span, trace};
use winit::event::ElementState;
use winit::keyboard::{KeyCode, PhysicalKey};

use crate::passes::{enter_span, merge_state_up};
use crate::render_root::RenderRoot;
use crate::{AccessEvent, EventCtx, Handled, PointerEvent, TextEvent, Widget, WidgetId};

// --- MARK: HELPERS ---
fn get_pointer_target(
    root: &RenderRoot,
    pointer_pos: Option<LogicalPosition<f64>>,
) -> Option<WidgetId> {
    if let Some(capture_target) = root.global_state.pointer_capture_target {
        return Some(capture_target);
    }

    if let Some(pointer_pos) = pointer_pos {
        // TODO - Apply scale?
        let pointer_pos = (pointer_pos.x, pointer_pos.y).into();
        return root
            .get_root_widget()
            .find_widget_at_pos(pointer_pos)
            .map(|widget| widget.id());
    }

    None
}

fn run_event_pass<E>(
    root: &mut RenderRoot,
    target: Option<WidgetId>,
    event: &E,
    allow_pointer_capture: bool,
    pass_fn: impl FnMut(&mut dyn Widget, &mut EventCtx, &E),
    trace: bool,
) -> Handled {
    let mut pass_fn = pass_fn;

    let original_target = target;
    let mut target_widget_id = target;
    let mut is_handled = false;
    while let Some(widget_id) = target_widget_id {
        let parent_id = root.widget_arena.parent_of(widget_id);
        let (mut widget_mut, mut state_mut) = root.widget_arena.get_pair_mut(widget_id);

        if !is_handled {
            let _span = enter_span(
                &root.global_state,
                widget_mut.reborrow(),
                state_mut.reborrow(),
            );
            let mut ctx = EventCtx {
                global_state: &mut root.global_state,
                widget_state: state_mut.item,
                widget_state_children: state_mut.children,
                widget_children: widget_mut.children,
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

            pass_fn(widget, &mut ctx, event);
            is_handled = ctx.is_handled;
        }

        merge_state_up(&mut root.widget_arena, widget_id);
        target_widget_id = parent_id;
    }

    Handled::from(is_handled)
}

// --- MARK: POINTER_EVENT ---
pub(crate) fn run_on_pointer_event_pass(root: &mut RenderRoot, event: &PointerEvent) -> Handled {
    let _span = info_span!("dispatch_pointer_event").entered();

    if event.is_high_density() {
        // We still want to record that this pass occurred in the debug file log.
        // However, we choose not to record any other tracing for this event,
        // as that would create a lot of noise.
        trace!("Running ON_POINTER_EVENT pass with {}", event.short_name());
    } else {
        debug!("Running ON_POINTER_EVENT pass with {}", event.short_name());
    }

    if event.position() != root.last_mouse_pos {
        root.global_state.needs_pointer_pass = true;
        root.last_mouse_pos = event.position();
    }

    let target_widget_id = get_pointer_target(root, event.position());

    let handled = run_event_pass(
        root,
        target_widget_id,
        event,
        matches!(event, PointerEvent::PointerDown(..)),
        |widget, ctx, event| {
            widget.on_pointer_event(ctx, event);
        },
        !event.is_high_density(),
    );

    if matches!(
        event,
        PointerEvent::PointerUp(..) | PointerEvent::PointerLeave(..)
    ) {
        // Automatically release the pointer on pointer up or leave. If a widget holds the capture,
        // it is notified of the pointer event before the capture is released, so it knows it is
        // about to lose the pointer.
        root.global_state.pointer_capture_target = None;
    }

    if !event.is_high_density() {
        debug!(
            focused_widget = root.global_state.focused_widget.map(|id| id.0),
            handled = handled.is_handled(),
            "ON_POINTER_EVENT finished",
        );
    }

    handled
}

// --- MARK: TEXT EVENT ---
pub(crate) fn run_on_text_event_pass(root: &mut RenderRoot, event: &TextEvent) -> Handled {
    if matches!(event, TextEvent::FocusChange(false)) {
        run_on_pointer_event_pass(root, &PointerEvent::new_pointer_leave());
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

    let target = root.global_state.focused_widget;

    let mut handled = run_event_pass(
        root,
        target,
        event,
        false,
        |widget, ctx, event| {
            widget.on_text_event(ctx, event);
        },
        !event.is_high_density(),
    );

    // Handle Tab focus
    if let TextEvent::KeyboardKey(key, mods) = event {
        if key.physical_key == PhysicalKey::Code(KeyCode::Tab)
            && key.state == ElementState::Pressed
            && handled == Handled::No
        {
            if !mods.shift_key() {
                root.global_state.next_focused_widget = root.widget_from_focus_chain(true);
            } else {
                root.global_state.next_focused_widget = root.widget_from_focus_chain(false);
            }
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
        |widget, ctx, event| {
            widget.on_access_event(ctx, event);
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
