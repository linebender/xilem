// Copyright 2024 the Xilem Authors
// SPDX-License-Identifier: Apache-2.0

use accesskit::{Node, Role};
use smallvec::{smallvec, SmallVec};
use tracing::{trace_span, Span};
use vello::kurbo::Point;
use vello::Scene;

use crate::core::{
    AccessCtx, AccessEvent, BoxConstraints, EventCtx, FromDynWidget, LayoutCtx, PaintCtx,
    PointerEvent, QueryCtx, RegisterCtx, TextEvent, Widget, WidgetId, WidgetMut, WidgetPod,
};
use crate::kurbo::Size;

// TODO: This is a hack to provide an accessibility node with a Window type.
// This should eventually be removed.
pub struct RootWidget<W: ?Sized> {
    pub(crate) pod: WidgetPod<W>,
}

impl<W: Widget> RootWidget<W> {
    pub fn new(widget: W) -> Self {
        Self {
            pod: WidgetPod::new(widget),
        }
    }
}

impl<W: Widget + FromDynWidget + ?Sized> RootWidget<W> {
    pub fn from_pod(pod: WidgetPod<W>) -> Self {
        Self { pod }
    }
}

impl<W: Widget + FromDynWidget + ?Sized> RootWidget<W> {
    pub fn child_mut<'t>(this: &'t mut WidgetMut<'_, Self>) -> WidgetMut<'t, W> {
        this.ctx.get_mut(&mut this.widget.pod)
    }
}

impl<W: Widget + FromDynWidget + ?Sized> Widget for RootWidget<W> {
    fn on_pointer_event(&mut self, _ctx: &mut EventCtx, _event: &PointerEvent) {}
    fn on_text_event(&mut self, _ctx: &mut EventCtx, _event: &TextEvent) {}
    fn on_access_event(&mut self, _ctx: &mut EventCtx, _event: &AccessEvent) {}

    fn register_children(&mut self, ctx: &mut RegisterCtx) {
        ctx.register_child(&mut self.pod);
    }

    fn layout(&mut self, ctx: &mut LayoutCtx, bc: &BoxConstraints) -> Size {
        let size = ctx.run_layout(&mut self.pod, bc);
        ctx.place_child(&mut self.pod, Point::ORIGIN);
        size
    }

    fn paint(&mut self, _ctx: &mut PaintCtx, _scene: &mut Scene) {}

    fn accessibility_role(&self) -> Role {
        Role::Window
    }

    fn accessibility(&mut self, _ctx: &mut AccessCtx, _node: &mut Node) {}

    fn children_ids(&self) -> SmallVec<[WidgetId; 16]> {
        smallvec![self.pod.id()]
    }

    fn make_trace_span(&self, ctx: &QueryCtx<'_>) -> Span {
        trace_span!("RootWidget", id = ctx.widget_id().trace())
    }
}
