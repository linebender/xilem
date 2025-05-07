// Copyright 2024 the Xilem Authors
// SPDX-License-Identifier: Apache-2.0

use accesskit::{Node, Role};
use smallvec::{SmallVec, smallvec};
use tracing::{Span, trace_span};
use vello::Scene;
use vello::kurbo::Point;

use crate::core::{
    AccessCtx, AccessEvent, BoxConstraints, EventCtx, FromDynWidget, LayoutCtx, PaintCtx,
    PointerEvent, PropertiesMut, PropertiesRef, QueryCtx, RegisterCtx, TextEvent, Widget, WidgetId,
    WidgetMut, WidgetPod,
};
use crate::kurbo::Size;

// TODO: This should eventually be removed once accesskit does that for us.
// See https://github.com/AccessKit/accesskit/issues/531
/// A widget wrapper that reports a [`Role::Window`] to the accessibility API.
pub struct RootWidget<W: ?Sized> {
    pub(crate) pod: WidgetPod<W>,
}

impl<W: Widget> RootWidget<W> {
    /// Create a new root widget.
    pub fn new(widget: W) -> Self {
        Self {
            pod: WidgetPod::new(widget),
        }
    }
}

impl<W: Widget + FromDynWidget + ?Sized> RootWidget<W> {
    /// Create a new root widget from a [`WidgetPod`].
    pub fn from_pod(pod: WidgetPod<W>) -> Self {
        Self { pod }
    }
}

impl<W: Widget + FromDynWidget + ?Sized> RootWidget<W> {
    /// Get a mutable reference to the child widget.
    pub fn child_mut<'t>(self: &'t mut WidgetMut<'_, Self>) -> WidgetMut<'t, W> {
        self.ctx.get_mut(&mut self.widget.pod)
    }
}

impl<W: Widget + FromDynWidget + ?Sized> Widget for RootWidget<W> {
    fn on_pointer_event(
        &mut self,
        _ctx: &mut EventCtx,
        _props: &mut PropertiesMut<'_>,
        _event: &PointerEvent,
    ) {
    }
    fn on_text_event(
        &mut self,
        _ctx: &mut EventCtx,
        _props: &mut PropertiesMut<'_>,
        _event: &TextEvent,
    ) {
    }
    fn on_access_event(
        &mut self,
        _ctx: &mut EventCtx,
        _props: &mut PropertiesMut<'_>,
        _event: &AccessEvent,
    ) {
    }

    fn register_children(&mut self, ctx: &mut RegisterCtx) {
        ctx.register_child(&mut self.pod);
    }

    fn layout(
        &mut self,
        ctx: &mut LayoutCtx,
        _props: &mut PropertiesMut<'_>,
        bc: &BoxConstraints,
    ) -> Size {
        let size = ctx.run_layout(&mut self.pod, bc);
        ctx.place_child(&mut self.pod, Point::ORIGIN);
        size
    }

    fn paint(&mut self, _ctx: &mut PaintCtx, _props: &PropertiesRef<'_>, _scene: &mut Scene) {}

    fn accessibility_role(&self) -> Role {
        Role::Window
    }

    fn accessibility(
        &mut self,
        _ctx: &mut AccessCtx,
        _props: &PropertiesRef<'_>,
        _node: &mut Node,
    ) {
    }

    fn children_ids(&self) -> SmallVec<[WidgetId; 16]> {
        smallvec![self.pod.id()]
    }

    fn make_trace_span(&self, ctx: &QueryCtx<'_>) -> Span {
        trace_span!("RootWidget", id = ctx.widget_id().trace())
    }
}
