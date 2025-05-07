// Copyright 2024 the Xilem Authors
// SPDX-License-Identifier: Apache-2.0

use std::any::TypeId;

use accesskit::{Node, Role};
use smallvec::{SmallVec, smallvec};
use tracing::{Span, trace_span};
use vello::Scene;
use vello::kurbo::Point;

use crate::core::{
    AccessCtx, AccessEvent, BoxConstraints, EventCtx, LayoutCtx, PaintCtx, PointerEvent,
    PropertiesMut, PropertiesRef, Property, QueryCtx, RegisterCtx, TextEvent, UpdateCtx, Widget,
    WidgetId, WidgetMut, WidgetPod,
};
use crate::kurbo::Size;
use crate::properties::{Background, Padding};
use crate::util::fill;

// TODO: This should eventually be removed once accesskit does that for us.
// See https://github.com/AccessKit/accesskit/issues/531
/// A widget wrapper that reports a [`Role::Window`] to the accessibility API.
pub struct RootWidget {
    pub(crate) pod: WidgetPod<dyn Widget>,
}

impl RootWidget {
    /// Create a new root widget.
    pub fn new(widget: impl Widget) -> Self {
        Self {
            pod: WidgetPod::new(widget).erased(),
        }
    }

    /// Create a new root widget from a [`WidgetPod`].
    pub fn from_pod(pod: WidgetPod<dyn Widget>) -> Self {
        Self { pod }
    }
}

impl RootWidget {
    /// Get a mutable reference to the child widget.
    pub fn child_mut<'t>(this: &'t mut WidgetMut<'_, Self>) -> WidgetMut<'t, dyn Widget> {
        this.ctx.get_mut(&mut this.widget.pod)
    }
}

impl Widget for RootWidget {
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

    fn property_changed(&mut self, ctx: &mut UpdateCtx, property_type: TypeId) {
        Background::prop_changed(ctx, property_type);
        Padding::prop_changed(ctx, property_type);
    }

    fn layout(
        &mut self,
        ctx: &mut LayoutCtx,
        props: &mut PropertiesMut<'_>,
        bc: &BoxConstraints,
    ) -> Size {
        let padding = props.get::<Padding>().unwrap_or(&Property::DEFAULT);

        let bc = padding.layout_down(*bc);
        let size = ctx.run_layout(&mut self.pod, &bc);
        let (size, _) = padding.layout_up(size, 0.);

        let pos = padding.place_down(Point::ORIGIN);
        ctx.place_child(&mut self.pod, pos);
        size
    }

    fn paint(&mut self, ctx: &mut PaintCtx, props: &PropertiesRef<'_>, scene: &mut Scene) {
        if let Some(bg) = props.get::<Background>() {
            let bg_rect = ctx.size().to_rect();
            let bg_brush = bg.get_peniko_brush_for_rect(bg_rect);

            fill(scene, &bg_rect, &bg_brush);
        }
    }

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
