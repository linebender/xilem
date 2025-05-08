// Copyright 2024 the Xilem Authors
// SPDX-License-Identifier: Apache-2.0

use accesskit::{Node, Role};
use smallvec::{SmallVec, smallvec};
use tracing::{Span, trace_span};
use vello::Scene;
use vello::kurbo::Point;

use crate::core::{
    AccessCtx, AccessEvent, BoxConstraints, EventCtx, LayoutCtx, PaintCtx, PointerEvent,
    PropertiesMut, PropertiesRef, QueryCtx, RegisterCtx, TextEvent, Widget, WidgetId, WidgetMut,
    WidgetPod,
};
use crate::kurbo::Size;

/// A wrapper Widget which app drivers can wrap around the rest of the widget tree.
///
/// This is useful for a few things:
/// - Reporting a [`Role::Window`] to the accessibility API.
/// - Setting a default [`Background`] and [`Padding`] for the entire app using [`DefaultProperties`].
///
/// [`DefaultProperties`]: crate::core::DefaultProperties
/// [`Background`]: crate::properties::Background
/// [`Padding`]: crate::properties::Padding
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
