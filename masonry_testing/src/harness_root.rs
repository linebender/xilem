// Copyright 2025 the Xilem Authors
// SPDX-License-Identifier: Apache-2.0

use std::any::TypeId;

use tracing::{Span, trace_span};

use masonry_core::accesskit::{Node, Role};
use masonry_core::core::{
    AccessCtx, AccessEvent, BoxConstraints, ChildrenIds, ComposeCtx, EventCtx, LayoutCtx,
    NewWidget, NoAction, PaintCtx, PointerEvent, PropertiesMut, PropertiesRef, RegisterCtx,
    TextEvent, Update, UpdateCtx, Widget, WidgetId, WidgetMut, WidgetPod,
};
use masonry_core::kurbo::{Point, Size};
use masonry_core::vello::Scene;

/// A basic widget which adds some padding around its child.
pub(crate) struct HarnessRoot {
    child: WidgetPod<dyn Widget>,
    padding: f64,
}

impl HarnessRoot {
    /// Create a new `HarnessRoot`.
    pub(crate) fn new(child: NewWidget<impl Widget>, padding: f64) -> Self {
        Self {
            child: child.erased().to_pod(),
            padding,
        }
    }

    /// Get mutable reference to the child widget.
    pub(crate) fn child_mut<'t>(this: &'t mut WidgetMut<'_, Self>) -> WidgetMut<'t, dyn Widget> {
        this.ctx.get_mut(&mut this.widget.child)
    }
}

impl Widget for HarnessRoot {
    type Action = NoAction;

    fn on_pointer_event(
        &mut self,
        _ctx: &mut EventCtx<'_>,
        _props: &mut PropertiesMut<'_>,
        _event: &PointerEvent,
    ) {
    }

    fn on_text_event(
        &mut self,
        _ctx: &mut EventCtx<'_>,
        _props: &mut PropertiesMut<'_>,
        _event: &TextEvent,
    ) {
    }

    fn on_access_event(
        &mut self,
        _ctx: &mut EventCtx<'_>,
        _props: &mut PropertiesMut<'_>,
        _event: &AccessEvent,
    ) {
    }

    fn register_children(&mut self, ctx: &mut RegisterCtx<'_>) {
        ctx.register_child(&mut self.child);
    }

    fn update(
        &mut self,
        _ctx: &mut UpdateCtx<'_>,
        _props: &mut PropertiesMut<'_>,
        _event: &Update,
    ) {
    }

    fn property_changed(&mut self, _ctx: &mut UpdateCtx<'_>, _property_type: TypeId) {}

    fn layout(
        &mut self,
        ctx: &mut LayoutCtx<'_>,
        _props: &mut PropertiesMut<'_>,
        bc: &BoxConstraints,
    ) -> Size {
        let pad = self.padding;

        let bc = bc.shrink((pad * 2., pad * 2.));
        let size = ctx.run_layout(&mut self.child, &bc);
        let size = Size::new(size.width + pad * 2., size.height + pad * 2.);

        ctx.place_child(&mut self.child, Point::new(pad, pad));
        size
    }

    fn compose(&mut self, _ctx: &mut ComposeCtx<'_>) {}

    fn paint(&mut self, _ctx: &mut PaintCtx<'_>, _props: &PropertiesRef<'_>, _scene: &mut Scene) {}

    fn accessibility_role(&self) -> Role {
        Role::GenericContainer
    }

    fn accessibility(
        &mut self,
        _ctx: &mut AccessCtx<'_>,
        _props: &PropertiesRef<'_>,
        _node: &mut Node,
    ) {
    }

    fn children_ids(&self) -> ChildrenIds {
        ChildrenIds::from_slice(&[self.child.id()])
    }

    fn make_trace_span(&self, id: WidgetId) -> Span {
        trace_span!("HarnessRoot", id = id.trace())
    }
}
