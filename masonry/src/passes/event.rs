// Copyright 2024 the Xilem Authors
// SPDX-License-Identifier: Apache-2.0

use dpi::LogicalPosition;
use tracing::{debug, info_span, trace};
use winit::keyboard::{KeyCode, PhysicalKey};

use crate::passes::merge_state_up;
use crate::render_root::RenderRoot;
use crate::{
    AccessEvent, EventCtx, Handled, PointerEvent, TextEvent, Widget, WidgetId, WidgetState,
};

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
    root_state: &mut WidgetState,
    target: Option<WidgetId>,
    event: &E,
    allow_pointer_capture: bool,
    pass_fn: impl FnMut(&mut dyn Widget, &mut EventCtx, &E),
) -> Handled {
    let mut pass_fn = pass_fn;

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
            allow_pointer_capture,
            is_handled: false,
            request_pan_to_child: None,
        };
        let widget = widget_mut.item;

        if !is_handled {
            trace!(
                "Widget '{}' #{} visited",
                widget.short_type_name(),
                widget_id.to_raw(),
            );

            pass_fn(widget, &mut ctx, event);
            is_handled = ctx.is_handled;
        }

        merge_state_up(&mut root.widget_arena, widget_id);
        target_widget_id = parent_id;
    }

    // Pass root widget state to synthetic state create at beginning of pass
    root_state.merge_up(root.widget_arena.get_state_mut(root.root.id()).item);

    Handled::from(is_handled)
}

// ----------------

// TODO - Send synthetic MouseLeave events

pub fn root_on_pointer_event(
    root: &mut RenderRoot,
    root_state: &mut WidgetState,
    event: &PointerEvent,
) -> Handled {
    let _span = info_span!("pointer_event").entered();
    if !event.is_high_density() {
        debug!("Running ON_POINTER_EVENT pass with {}", event.short_name());
    }

    root.last_mouse_pos = event.position();

    let target_widget_id = get_target_widget(root, event.position());

    let handled = run_event_pass(
        root,
        root_state,
        target_widget_id,
        event,
        matches!(event, PointerEvent::PointerDown(..)),
        |widget, ctx, event| {
            widget.on_pointer_event(ctx, event);
        },
    );

    if !event.is_high_density() {
        debug!(
            focused_widget = root.state.focused_widget.map(|id| id.0),
            handled = handled.is_handled(),
            "ON_POINTER_EVENT finished",
        );
    }

    handled
}

pub fn root_on_text_event(
    root: &mut RenderRoot,
    root_state: &mut WidgetState,
    event: &TextEvent,
) -> Handled {
    let _span = info_span!("text_event").entered();
    if !event.is_high_density() {
        debug!("Running ON_TEXT_EVENT pass with {}", event.short_name());
    }

    let target = root.state.focused_widget;

    let mut handled = run_event_pass(
        root,
        root_state,
        target,
        event,
        false,
        |widget, ctx, event| {
            widget.on_text_event(ctx, event);
        },
    );

    // Handle Tab focus
    if let TextEvent::KeyboardKey(key, mods) = event {
        if handled == Handled::No && key.physical_key == PhysicalKey::Code(KeyCode::Tab) {
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

pub fn root_on_access_event(
    root: &mut RenderRoot,
    root_state: &mut WidgetState,
    event: &AccessEvent,
) -> Handled {
    let _span = info_span!("access_event").entered();
    debug!("Running ON_ACCESS_EVENT pass with {}", event.short_name());

    let target = Some(event.target);

    let handled = run_event_pass(
        root,
        root_state,
        target,
        event,
        false,
        |widget, ctx, event| {
            widget.on_access_event(ctx, event);
        },
    );

    debug!(
        focused_widget = root.state.focused_widget.map(|id| id.0),
        handled = handled.is_handled(),
        "ON_ACCESS_EVENT finished",
    );

    handled
}

// These functions were carved out of WidgetPod code during a previous refactor
// The general "pan to child" logic needs to be added back in.
#[cfg(FALSE)]
fn pan_to_child() {
    // TODO - there's some dubious logic here
    if let Some(target_rect) = inner_ctx.request_pan_to_child {
        self.pan_to_child(parent_ctx, target_rect);
        let (state, _) = parent_ctx
            .widget_state_children
            .get_child_mut(id)
            .expect("WidgetPod: inner widget not found in widget tree");
        let new_rect = target_rect.with_origin(target_rect.origin() + state.origin.to_vec2());
        parent_ctx.request_pan_to_child = Some(new_rect);
    }
}

#[cfg(FALSE)]
fn pan_to_child(&mut self, parent_ctx: &mut EventCtx, rect: Rect) {
    let id = self.id().to_raw();
    let (widget, widget_token) = parent_ctx
        .widget_children
        .get_child_mut(id)
        .expect("WidgetPod: inner widget not found in widget tree");
    let (state, state_token) = parent_ctx
        .widget_state_children
        .get_child_mut(id)
        .expect("WidgetPod: inner widget not found in widget tree");
    let mut inner_ctx = LifeCycleCtx {
        global_state: parent_ctx.global_state,
        widget_state: state,
        widget_state_children: state_token,
        widget_children: widget_token,
    };
    let event = LifeCycle::RequestPanToChild(rect);

    widget.lifecycle(&mut inner_ctx, &event);
}
