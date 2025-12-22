// Copyright 2025 the Xilem Authors
// SPDX-License-Identifier: Apache-2.0

use std::any::TypeId;

use masonry_core::accesskit::{Node, Role};
use masonry_core::core::{
    AccessCtx, AccessEvent, BoxConstraints, ChildrenIds, ComposeCtx, EventCtx, LayoutCtx,
    NewWidget, NoAction, PaintCtx, PointerEvent, PropertiesMut, PropertiesRef, RegisterCtx,
    TextEvent, Update, UpdateCtx, Widget, WidgetMut, WidgetPod,
};
use masonry_core::kurbo::{Point, Size};
use masonry_core::vello::Scene;

/// A basic wrapper widget that can replace its child.
pub struct WrapperWidget {
    child: WidgetPod<dyn Widget>,
}

impl WrapperWidget {
    /// Creates a new `WrapperWidget`.
    ///
    /// The `child` is the initial child widget.
    pub fn new(child: NewWidget<impl Widget>) -> Self {
        Self {
            child: child.erased().to_pod(),
        }
    }

    /// Returns mutable reference to the child widget.
    pub fn child_mut<'t>(this: &'t mut WidgetMut<'_, Self>) -> WidgetMut<'t, dyn Widget> {
        this.ctx.get_mut(&mut this.widget.child)
    }
}

impl WrapperWidget {
    /// Replaces the container's child widget.
    pub fn set_child(this: &mut WidgetMut<'_, Self>, child: impl Widget) {
        // FIXME - Take NewWidget argument
        Self::set_child_pod(this, NewWidget::new(child).erased().to_pod());
    }

    /// Replaces the container's child widget with a `WidgetPod`.
    pub fn set_child_pod(this: &mut WidgetMut<'_, Self>, child: WidgetPod<dyn Widget>) {
        let old_child = std::mem::replace(&mut this.widget.child, child);
        this.ctx.remove_child(old_child);
    }
}

impl Widget for WrapperWidget {
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
        let size = ctx.run_layout(&mut self.child, bc);
        ctx.place_child(&mut self.child, Point::ORIGIN);
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
}
