// Copyright 2024 the Xilem Authors and the Druid Authors
// SPDX-License-Identifier: Apache-2.0

//! A titlebar area widget.

use accesskit::{NodeBuilder, Role};
use dpi::LogicalPosition;
use smallvec::{smallvec, SmallVec};
use tracing::{trace_span, Span};
use vello::kurbo::Point;
use vello::Scene;

use crate::event::PointerButton;
use crate::widget::{WidgetMut, WidgetPod};
use crate::{
    AccessCtx, AccessEvent, BoxConstraints, EventCtx, LayoutCtx, LifeCycleCtx, PaintCtx,
    PointerEvent, Size, StatusChange, TextEvent, Widget, WidgetId,
};

// TODO: Maybe make this configurable somehow
const DRAG_THRESHOLD_SQUARED: f64 = 8.0;

/// A titlebar area widget.
///
/// A draggable area that allows moving the window, useful to create a custom titlebar widget.
/// The child can be used to include typical titlebar elements, such as the window title and buttons.
///
/// Note that this widget does not set a fixed size, background color or accessibility role.
/// It's not intended as a complete titlebar solution. To achieve this, you should create a custom
/// widget that handles sizing, styling and accessibility, then wrap it with the `WindowHandle`.
///
/// This widget is inspired by the [GtkWindowHandle](https://docs.gtk.org/gtk4/class.WindowHandle.html) from GTK.
pub struct WindowHandle<W: Widget> {
    child: Option<WidgetPod<W>>,
    pointer_down_pos: Option<LogicalPosition<f64>>,
}

// --- MARK: BUILDERS ---
impl<W: Widget> WindowHandle<W> {
    pub fn new(child: W) -> Self {
        Self {
            child: Some(WidgetPod::new(child)),
            pointer_down_pos: None,
        }
    }

    pub fn empty() -> Self {
        Self {
            child: None,
            pointer_down_pos: None,
        }
    }
}

// --- MARK: WIDGETMUT ---
impl<W: Widget> WidgetMut<'_, WindowHandle<W>> {
    pub fn set_child(&mut self, child: W) {
        if let Some(child) = self.widget.child.take() {
            self.ctx.remove_child(child);
        }
        self.widget.child = Some(WidgetPod::new(child));
        self.ctx.children_changed();
        self.ctx.request_layout();
    }

    pub fn remove_child(&mut self) {
        if let Some(child) = self.widget.child.take() {
            self.ctx.remove_child(child);
        }
    }
}

// --- MARK: IMPL WIDGET ---
impl<W: Widget> Widget for WindowHandle<W> {
    fn on_pointer_event(&mut self, ctx: &mut EventCtx, event: &PointerEvent) {
        match event {
            PointerEvent::PointerDown(button, state) => {
                if !ctx.is_disabled() {
                    // TODO: Add double click to maximize
                    match button {
                        PointerButton::Primary => self.pointer_down_pos = Some(state.position),
                        PointerButton::Secondary => ctx.show_window_menu(state.position),
                        _ => (),
                    }
                }
            }
            PointerEvent::PointerMove(state) => {
                if let Some(last_pos) = self.pointer_down_pos {
                    let distance_squared = (state.position.x - last_pos.x).powi(2)
                        + (state.position.y - last_pos.y).powi(2);

                    if distance_squared >= DRAG_THRESHOLD_SQUARED {
                        ctx.drag_window();
                        self.pointer_down_pos = None;
                    }
                }
            }
            PointerEvent::PointerLeave(_) | PointerEvent::PointerUp(_, _) => {
                self.pointer_down_pos = None;
            }
            _ => (),
        }
    }

    fn on_text_event(&mut self, _ctx: &mut EventCtx, _event: &TextEvent) {}

    fn on_access_event(&mut self, _ctx: &mut EventCtx, _event: &AccessEvent) {}

    fn on_status_change(&mut self, _ctx: &mut LifeCycleCtx, _event: &StatusChange) {}

    fn register_children(&mut self, ctx: &mut crate::RegisterCtx) {
        if let Some(ref mut child) = self.child {
            ctx.register_child(child);
        }
    }

    fn layout(&mut self, ctx: &mut LayoutCtx, bc: &BoxConstraints) -> Size {
        match self.child.as_mut() {
            Some(child) => {
                let size = ctx.run_layout(child, bc);
                ctx.place_child(child, Point::ORIGIN);
                bc.constrain(size)
            }
            None => bc.max(),
        }
    }

    fn paint(&mut self, _ctx: &mut PaintCtx, _scene: &mut Scene) {}

    fn accessibility_role(&self) -> Role {
        Role::GenericContainer
    }

    fn accessibility(&mut self, _ctx: &mut AccessCtx, _node: &mut NodeBuilder) {}

    fn children_ids(&self) -> SmallVec<[WidgetId; 16]> {
        if let Some(child) = &self.child {
            smallvec![child.id()]
        } else {
            SmallVec::new()
        }
    }

    fn make_trace_span(&self) -> Span {
        trace_span!("WindowHandle")
    }
}
