// Copyright 2024 the Xilem Authors
// SPDX-License-Identifier: Apache-2.0

use cursor_icon::CursorIcon;
use dpi::LogicalPosition;
use tracing::{debug, info_span, trace};
use winit::keyboard::{KeyCode, PhysicalKey};

use crate::render_root::{RenderRoot, RenderRootSignal, WidgetArena};
use crate::tree_arena::ArenaMutChildren;
use crate::{
    AccessEvent, EventCtx, Handled, PointerEvent, TextEvent, Widget, WidgetId, WidgetState,
};

// References shared by all passes
struct PassCtx<'a> {
    pub(crate) widget_state: &'a mut WidgetState,
    pub(crate) widget_state_children: ArenaMutChildren<'a, WidgetState>,
    pub(crate) widget_children: ArenaMutChildren<'a, Box<dyn Widget>>,
}

impl<'a> PassCtx<'a> {
    fn parent(&self) -> Option<WidgetId> {
        let parent_id = self.widget_children.parent_id()?;
        let parent_id = parent_id.try_into().unwrap();
        Some(WidgetId(parent_id))
    }
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
            widget_state: state_mut.item,
            widget_state_children: state_mut.children,
            widget_children: widget_mut.children,
        },
    )
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

// ----------------

// TODO - Handle hover status
// TODO - Handle pointer capture
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
        |widget, ctx, event| {
            widget.on_pointer_event(ctx, event);
        },
    );

    // Update cursor depending on pointed widget
    if let Some(cursor) = &root_state.cursor {
        // TODO - Add methods and `into()` impl to make this more concise.
        root.state
            .signal_queue
            .push_back(RenderRootSignal::SetCursor(*cursor));
    } else {
        root.state
            .signal_queue
            .push_back(RenderRootSignal::SetCursor(CursorIcon::Default));
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

    let mut handled = run_event_pass(root, root_state, target, event, |widget, ctx, event| {
        widget.on_text_event(ctx, event);
    });

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

    let handled = run_event_pass(root, root_state, target, event, |widget, ctx, event| {
        widget.on_access_event(ctx, event);
    });

    debug!(
        focused_widget = root.state.focused_widget.map(|id| id.0),
        handled = handled.is_handled(),
        "ON_ACCESS_EVENT finished",
    );

    handled
}

// ---

fn run_event_pass<E>(
    root: &mut RenderRoot,
    root_state: &mut WidgetState,
    target: Option<WidgetId>,
    event: &E,
    pass_fn: impl FnMut(&mut dyn Widget, &mut EventCtx, &E),
) -> Handled {
    let mut pass_fn = pass_fn;

    let mut target_widget_id = target;
    let mut is_handled = false;
    while let Some(widget_id) = target_widget_id {
        let (widget, pass_ctx) = get_widget_mut(&mut root.widget_arena, widget_id);
        let parent_id = pass_ctx.parent();

        let mut ctx = EventCtx {
            global_state: &mut root.state,
            widget_state: pass_ctx.widget_state,
            widget_state_children: pass_ctx.widget_state_children,
            widget_children: pass_ctx.widget_children,
            is_handled: false,
            request_pan_to_child: None,
        };

        if !is_handled {
            trace!(
                "Widget '{}' #{} visited",
                widget.short_type_name(),
                widget_id.to_raw(),
            );

            pass_fn(widget, &mut ctx, event);
            is_handled = ctx.is_handled;
        }

        merge_state_up(&mut root.widget_arena, widget_id, root_state);
        target_widget_id = parent_id;
    }

    Handled::from(is_handled)
}

#[cfg(FALSE)]
fn on_pointer_event() {
    let hot_changed = true;

    let call_widget = (had_active || state.is_hot || hot_changed) && !state.is_stashed;
    if call_widget {
        let mut inner_ctx = EventCtx {
            global_state: parent_ctx.global_state,
            widget_state: state,
            widget_state_children: state_token,
            widget_children: widget_token,
            is_handled: false,
            request_pan_to_child: None,
        };
        inner_ctx.widget_state.has_active = false;

        widget.on_pointer_event(&mut inner_ctx, event);

        inner_ctx.widget_state.has_active |= inner_ctx.widget_state.is_active;
        parent_ctx.is_handled |= inner_ctx.is_handled;
    }

    call_widget
}

#[cfg(FALSE)]
fn hot_state() {
    // TODO - This doesn't handle the case where multiple cursors
    // are over the same widget
    let hot_pos = match event {
        PointerEvent::PointerDown(_, pointer_state) => Some(pointer_state.position),
        PointerEvent::PointerUp(_, pointer_state) => Some(pointer_state.position),
        PointerEvent::PointerMove(pointer_state) => Some(pointer_state.position),
        PointerEvent::PointerEnter(pointer_state) => Some(pointer_state.position),
        PointerEvent::PointerLeave(_) => None,
        PointerEvent::MouseWheel(_, pointer_state) => Some(pointer_state.position),
        PointerEvent::HoverFile(_, _) => None,
        PointerEvent::DropFile(_, _) => None,
        PointerEvent::HoverFileCancel(_) => None,
    };
    let hot_changed = WidgetPod::update_hot_state(
        self.id(),
        widget.as_mut_dyn_any().downcast_mut::<W>().unwrap(),
        widget_token.reborrow_mut(),
        state,
        state_token.reborrow_mut(),
        parent_ctx.global_state,
        hot_pos,
    );

    let call_widget = (had_active || state.is_hot || hot_changed) && !state.is_stashed;
    if call_widget {
        // stuff
    }
}

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
