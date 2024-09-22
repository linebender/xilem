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

const DRAG_OFFSET: f64 = 5.0;

/// A titlebar area widget.
///
/// An area that can be dragged to move the window.
pub struct WindowHandle {
    child: Option<WidgetPod<Box<dyn Widget>>>,
    last_pos: Option<LogicalPosition<f64>>,
}

// --- MARK: BUILDERS ---
impl WindowHandle {
    pub fn new(child: impl Widget) -> Self {
        Self {
            child: Some(WidgetPod::new(child).boxed()),
            last_pos: None,
        }
    }
}

// --- MARK: WIDGETMUT ---
impl WidgetMut<'_, WindowHandle> {
    pub fn set_child(&mut self, child: impl Widget) {
        if let Some(child) = self.widget.child.take() {
            self.ctx.remove_child(child);
        }
        self.widget.child = Some(WidgetPod::new(child).boxed());
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
impl Widget for WindowHandle {
    fn on_pointer_event(&mut self, ctx: &mut EventCtx, event: &PointerEvent) {
        match event {
            PointerEvent::PointerDown(button, state) => {
                if !ctx.is_disabled() {
                    match button {
                        PointerButton::Primary => self.last_pos = Some(state.position),
                        PointerButton::Secondary => ctx.show_window_menu(state.position),
                        _ => (),
                    }
                }
            }
            PointerEvent::PointerMove(state) => {
                if let Some(last_pos) = self.last_pos {
                    let distance = ((state.position.x - last_pos.x).powi(2)
                        + (state.position.y - last_pos.y).powi(2))
                    .sqrt();

                    if distance >= DRAG_OFFSET {
                        ctx.drag_window();
                        self.last_pos = None;
                    }
                }
            }
            PointerEvent::PointerLeave(_) | PointerEvent::PointerUp(_, _) => {
                self.last_pos = None;
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
                size
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
