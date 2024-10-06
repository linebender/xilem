// Copyright 2024 the Xilem Authors and the Druid Authors
// SPDX-License-Identifier: Apache-2.0

//! A window button.

use accesskit::{NodeBuilder, Role};
use smallvec::{smallvec, SmallVec};
use tracing::{trace_span, Span};
use vello::kurbo::Point;
use vello::Scene;

use crate::widget::WidgetPod;
use crate::{
    AccessCtx, AccessEvent, BoxConstraints, EventCtx, LayoutCtx, LifeCycleCtx, PaintCtx,
    PointerEvent, Size, StatusChange, TextEvent, Widget, WidgetId,
};

use super::Label;

pub enum WindowButtonType {
    Close,
    Maximize,
    Minimize,
}

/// A window button.
pub struct WindowButton {
    child: WidgetPod<Label>,
    type_: WindowButtonType,
}

// --- MARK: BUILDERS ---
impl WindowButton {
    pub fn new(type_: WindowButtonType) -> Self {
        let handle = Label::new(match type_ {
            WindowButtonType::Close => "×",
            WindowButtonType::Maximize => "+",
            WindowButtonType::Minimize => "−",
        })
        .with_text_size(20.);

        Self {
            child: WidgetPod::new(handle),
            type_,
        }
    }
}

// --- MARK: IMPL WIDGET ---
impl Widget for WindowButton {
    fn on_pointer_event(&mut self, ctx: &mut EventCtx, event: &PointerEvent) {
        match event {
            PointerEvent::PointerDown(_, _) => {
                if !ctx.is_disabled() {
                    ctx.capture_pointer();
                }
            }
            PointerEvent::PointerUp(_, _) => match self.type_ {
                WindowButtonType::Close => ctx.exit(),
                WindowButtonType::Maximize => ctx.toggle_maximized(),
                WindowButtonType::Minimize => ctx.minimize(),
            },
            PointerEvent::PointerLeave(_) => {}
            _ => (),
        }
    }

    fn on_text_event(&mut self, _ctx: &mut EventCtx, _event: &TextEvent) {}

    fn on_access_event(&mut self, _ctx: &mut EventCtx, _event: &AccessEvent) {}

    fn on_status_change(&mut self, _ctx: &mut LifeCycleCtx, _event: &StatusChange) {}

    fn register_children(&mut self, ctx: &mut crate::RegisterCtx) {
        ctx.register_child(&mut self.child);
    }

    fn layout(&mut self, ctx: &mut LayoutCtx, bc: &BoxConstraints) -> Size {
        let size = ctx.run_layout(&mut self.child, bc);
        ctx.place_child(&mut self.child, Point::ORIGIN);
        size
    }

    fn paint(&mut self, _ctx: &mut PaintCtx, _scene: &mut Scene) {}

    fn accessibility_role(&self) -> Role {
        Role::Button
    }

    fn accessibility(&mut self, _ctx: &mut AccessCtx, _node: &mut NodeBuilder) {}

    fn children_ids(&self) -> SmallVec<[WidgetId; 16]> {
        smallvec![self.child.id()]
    }

    fn make_trace_span(&self) -> Span {
        trace_span!("WindowButton")
    }
}
