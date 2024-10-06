// Copyright 2024 the Xilem Authors and the Druid Authors
// SPDX-License-Identifier: Apache-2.0

//! A center box widget.

use accesskit::{NodeBuilder, Role};
use smallvec::SmallVec;
use tracing::{trace_span, Span};
use vello::kurbo::Point;
use vello::Scene;

use crate::widget::WidgetPod;
use crate::{
    AccessCtx, AccessEvent, BoxConstraints, EventCtx, LayoutCtx, LifeCycleCtx, PaintCtx,
    PointerEvent, Size, StatusChange, TextEvent, Widget, WidgetId,
};

const PADDING: f64 = 10.;

/// A center box widget.
pub struct CenterBox {
    child: Option<WidgetPod<Box<dyn Widget>>>,
    end: Option<WidgetPod<Box<dyn Widget>>>,
}

// --- MARK: BUILDERS ---
impl CenterBox {
    // TODO: Maybe the end widget should be optional here
    pub(crate) fn new(child: impl Widget, end: impl Widget) -> Self {
        Self {
            child: Some(WidgetPod::new(child).boxed()),
            end: Some(WidgetPod::new(end).boxed()),
        }
    }
}

// --- MARK: IMPL WIDGET ---
impl Widget for CenterBox {
    fn on_pointer_event(&mut self, _ctx: &mut EventCtx, _event: &PointerEvent) {}

    fn on_text_event(&mut self, _ctx: &mut EventCtx, _event: &TextEvent) {}

    fn on_access_event(&mut self, _ctx: &mut EventCtx, _event: &AccessEvent) {}

    fn on_status_change(&mut self, _ctx: &mut LifeCycleCtx, _event: &StatusChange) {}

    fn register_children(&mut self, ctx: &mut crate::RegisterCtx) {
        if let Some(ref mut child) = self.child {
            ctx.register_child(child);
        }

        if let Some(ref mut end) = self.end {
            ctx.register_child(end);
        }
    }

    fn layout(&mut self, ctx: &mut LayoutCtx, bc: &BoxConstraints) -> Size {
        let child = self.child.as_mut().unwrap();
        let end = self.end.as_mut().unwrap();

        let child_size = ctx.run_layout(child, &bc.loosen());
        let end_size = ctx.run_layout(end, &bc.loosen());

        let box_size = bc.constrain(Size::new(
            child_size.width + end_size.width,
            child_size.height.max(end_size.height),
        ));

        ctx.place_child(
            child,
            Point::new(
                (box_size.width / 2.0) - (child_size.width / 2.0),
                (box_size.height / 2.0) - (child_size.height / 2.0),
            ),
        );

        ctx.place_child(
            end,
            Point::new(
                box_size.width - end_size.width - PADDING,
                (box_size.height / 2.0) - (end_size.height / 2.0),
            ),
        );

        box_size
    }

    fn paint(&mut self, _ctx: &mut PaintCtx, _scene: &mut Scene) {}

    fn accessibility_role(&self) -> Role {
        Role::GenericContainer
    }

    fn accessibility(&mut self, _ctx: &mut AccessCtx, _node: &mut NodeBuilder) {}

    fn children_ids(&self) -> SmallVec<[WidgetId; 16]> {
        let mut vec = SmallVec::new();

        if let Some(child) = &self.child {
            vec.push(child.id());
        }

        if let Some(end) = &self.end {
            vec.push(end.id());
        }

        vec
    }

    fn make_trace_span(&self) -> Span {
        trace_span!("CenterBox")
    }
}
