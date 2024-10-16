// Copyright 2024 the Xilem Authors
// SPDX-License-Identifier: Apache-2.0

use dpi::LogicalPosition;
use tracing::{debug, info_span, trace};
use winit::event::ElementState;
use winit::keyboard::{KeyCode, PhysicalKey};

use crate::passes::merge_state_up;
use crate::render_root::RenderRoot;
use crate::{AccessEvent, EventCtx, Handled, PointerEvent, TextEvent, Widget, WidgetId};

// --- MARK: HELPERS ---
fn get_target_widget(
    root: &RenderRoot,
    pointer_pos: Option<LogicalPosition<f64>>,
) -> Option<WidgetId> {
    if let Some(capture_target) = root.state.pointer_capture_target {
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
) -> Handled {
    let mut pass_fn = pass_fn;

    let original_target = target;
    let mut target_widget_id = target;
    let mut is_handled = false;
    while let Some(widget_id) = target_widget_id {
        let parent_id = root.widget_arena.parent_of(widget_id);
        let (widget_mut, state_mut) = root.widget_arena.get_pair_mut(widget_id);

        let mut ctx = EventCtx {
            global_state: &mut root.state,
            widget_state: state_mut.item,
            widget_state_children: state_mut.children,
            widget_children: widget_mut.children,
            target: original_target.unwrap(),
            allow_pointer_capture,
            is_handled: false,
        };
        let widget = widget_mut.item;

        if !is_handled {
            trace!(
                "Widget '{}' {} visited",
                widget.short_type_name(),
                widget_id,
            );

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
    let _span = info_span!("pointer_event").entered();
    if !event.is_high_density() {
        debug!("Running ON_POINTER_EVENT pass with {}", event.short_name());
    }

    root.last_mouse_pos = event.position();

    let target_widget_id = get_target_widget(root, event.position());

    let handled = run_event_pass(
        root,
        target_widget_id,
        event,
        matches!(event, PointerEvent::PointerDown(..)),
        |widget, ctx, event| {
            widget.on_pointer_event(ctx, event);
        },
    );

    if matches!(
        event,
        PointerEvent::PointerUp(..) | PointerEvent::PointerLeave(..)
    ) {
        // Automatically release the pointer on pointer up or leave. If a widget holds the capture,
        // it is notified of the pointer event before the capture is released, so it knows it is
        // about to lose the pointer.
        root.state.pointer_capture_target = None;
    }

    if !event.is_high_density() {
        debug!(
            focused_widget = root.state.focused_widget.map(|id| id.0),
            handled = handled.is_handled(),
            "ON_POINTER_EVENT finished",
        );
    }

    handled
}

// TODO https://github.com/linebender/xilem/issues/376 - Some implicit invariants:
// - If a Widget gets a keyboard event or an ImeStateChange, then
// focus is on it, its child or its parent.
// - If a Widget has focus, then none of its parents is hidden

// --- MARK: TEXT EVENT ---
pub(crate) fn run_on_text_event_pass(root: &mut RenderRoot, event: &TextEvent) -> Handled {
    let _span = info_span!("text_event").entered();
    if !event.is_high_density() {
        debug!("Running ON_TEXT_EVENT pass with {}", event.short_name());
    }

    let target = root.state.focused_widget;

    let mut handled = run_event_pass(root, target, event, false, |widget, ctx, event| {
        widget.on_text_event(ctx, event);
    });

    // Handle Tab focus
    if let TextEvent::KeyboardKey(key, mods) = event {
        if key.physical_key == PhysicalKey::Code(KeyCode::Tab)
            && key.state == ElementState::Pressed
            && handled == Handled::No
        {
            if !mods.shift_key() {
                root.state.next_focused_widget = root.widget_from_focus_chain(true);
            } else {
                root.state.next_focused_widget = root.widget_from_focus_chain(false);
            }
            handled = Handled::Yes;
        }
    }

    if !event.is_high_density() {
        debug!(
            focused_widget = root.state.focused_widget.map(|id| id.0),
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

    let mut handled = run_event_pass(root, Some(target), event, false, |widget, ctx, event| {
        widget.on_access_event(ctx, event);
    });

    // Handle focus events
    match event.action {
        accesskit::Action::Focus if !handled.is_handled() => {
            if root.is_still_interactive(target) {
                root.state.next_focused_widget = Some(target);
                handled = Handled::Yes;
            }
        }
        accesskit::Action::Blur if !handled.is_handled() => {
            if root.state.next_focused_widget == Some(target) {
                root.state.next_focused_widget = None;
                handled = Handled::Yes;
            }
        }
        _ => {}
    }

    debug!(
        focused_widget = root.state.focused_widget.map(|id| id.0),
        handled = handled.is_handled(),
        "ON_ACCESS_EVENT finished",
    );

    handled
}
