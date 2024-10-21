// Copyright 2024 the Xilem Authors
// SPDX-License-Identifier: Apache-2.0

use accesskit::{NodeBuilder, Role};
use smallvec::{smallvec, SmallVec};
use tracing::{trace_span, Span};
use vello::kurbo::Point;
use vello::Scene;

use crate::widget::{BiAxial, ContentFill, WidgetMut, WidgetPod};
use crate::{
    AccessCtx, AccessEvent, BoxConstraints, EventCtx, LayoutCtx, PaintCtx, PointerEvent,
    RegisterCtx, Size, StatusChange, TextEvent, UpdateCtx, Widget, WidgetId,
};

// TODO: This is a hack to provide an accessibility node with a Window type.
// This should eventually be removed.
pub struct RootWidget<W> {
    pub(crate) pod: WidgetPod<W>,
}

impl<W: Widget> RootWidget<W> {
    pub fn new(widget: W) -> RootWidget<W> {
        RootWidget {
            pod: WidgetPod::new(widget),
        }
    }

    // TODO - This help works around impedance mismatch between the types of Xilem and Masonry
    pub fn from_pod(pod: WidgetPod<W>) -> RootWidget<W> {
        RootWidget { pod }
    }
}

impl<W: Widget> WidgetMut<'_, RootWidget<W>> {
    pub fn child_mut(&mut self) -> WidgetMut<'_, W> {
        self.ctx.get_mut(&mut self.widget.pod)
    }
}

impl<W: Widget> Widget for RootWidget<W> {
    fn on_pointer_event(&mut self, _ctx: &mut EventCtx, _event: &PointerEvent) {}
    fn on_text_event(&mut self, _ctx: &mut EventCtx, _event: &TextEvent) {}
    fn on_access_event(&mut self, _ctx: &mut EventCtx, _event: &AccessEvent) {}

    fn on_status_change(&mut self, _: &mut UpdateCtx, _: &StatusChange) {}

    fn register_children(&mut self, ctx: &mut RegisterCtx) {
        ctx.register_child(&mut self.pod);
    }

    fn layout(
        &mut self,
        ctx: &mut LayoutCtx,
        available_space: &BiAxial<f64>,
        requested_fill: &BiAxial<ContentFill>,
    ) -> BiAxial<f64> {
        let size = ctx.run_layout(&mut self.pod, available_space, requested_fill);
        ctx.place_child(&mut self.pod, Point::ORIGIN);
        size
    }

    fn paint(&mut self, _ctx: &mut PaintCtx, _scene: &mut Scene) {}

    fn accessibility_role(&self) -> Role {
        Role::Window
    }

    fn accessibility(&mut self, _ctx: &mut AccessCtx, _node: &mut NodeBuilder) {}

    fn children_ids(&self) -> SmallVec<[WidgetId; 16]> {
        smallvec![self.pod.id()]
    }

    fn make_trace_span(&self) -> Span {
        trace_span!("RootWidget")
    }
}
