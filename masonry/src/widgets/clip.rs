// Copyright 2025 the Xilem Authors
// SPDX-License-Identifier: Apache-2.0

//! A widget that clips its child to its own layout rect.

use accesskit::{Node, Role};
use tracing::{Span, trace_span};
use vello::Scene;

use crate::core::{
    AccessCtx, AccessEvent, ChildrenIds, EventCtx, FromDynWidget, LayoutCtx, MeasureCtx, NewWidget,
    NoAction, PaintCtx, PropertiesMut, PropertiesRef, RegisterCtx, TextEvent, Widget, WidgetId,
    WidgetMut, WidgetPod,
};
use crate::kurbo::{Axis, Point, Rect, Size};
use crate::layout::LenReq;

/// A container widget that clips its child to its own layout rect.
///
/// This is useful for preventing a child from painting (or receiving
/// pointer events) outside of its allocated space.
pub struct Clip<Child>
where
    Child: Widget + ?Sized,
{
    enabled: bool,
    child: WidgetPod<Child>,
}

// --- MARK: BUILDERS
impl<Child: Widget + ?Sized> Clip<Child> {
    /// Creates a new `Clip`.
    ///
    /// Clipping is enabled by default.
    pub fn new(child: NewWidget<Child>) -> Self {
        Self {
            enabled: true,
            child: child.to_pod(),
        }
    }

    /// Sets whether clipping is enabled.
    pub fn enabled(mut self, enabled: bool) -> Self {
        self.enabled = enabled;
        self
    }
}

// --- MARK: WIDGETMUT
impl<Child> Clip<Child>
where
    Child: Widget + FromDynWidget + ?Sized,
{
    /// Sets whether clipping is enabled.
    pub fn set_enabled(this: &mut WidgetMut<'_, Self>, enabled: bool) {
        this.widget.enabled = enabled;
        this.ctx.request_layout();
    }

    /// Returns a mutable reference to the child widget.
    pub fn child_mut<'t>(this: &'t mut WidgetMut<'_, Self>) -> WidgetMut<'t, Child> {
        this.ctx.get_mut(&mut this.widget.child)
    }

    /// Replaces the child widget.
    pub fn set_child(this: &mut WidgetMut<'_, Self>, child: NewWidget<Child>) {
        this.ctx
            .remove_child(std::mem::replace(&mut this.widget.child, child.to_pod()));
    }
}

// --- MARK: IMPL WIDGET
impl<Child> Widget for Clip<Child>
where
    Child: Widget + ?Sized,
{
    type Action = NoAction;

    fn on_pointer_event(
        &mut self,
        _ctx: &mut EventCtx<'_>,
        _props: &mut PropertiesMut<'_>,
        _event: &crate::core::PointerEvent,
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

    fn measure(
        &mut self,
        ctx: &mut MeasureCtx<'_>,
        _props: &PropertiesRef<'_>,
        axis: Axis,
        _len_req: LenReq,
        cross_length: Option<f64>,
    ) -> f64 {
        ctx.redirect_measurement(&mut self.child, axis, cross_length)
    }

    fn layout(&mut self, ctx: &mut LayoutCtx<'_>, _props: &PropertiesRef<'_>, size: Size) {
        ctx.run_layout(&mut self.child, size);
        ctx.place_child(&mut self.child, Point::ORIGIN);

        if self.enabled {
            ctx.set_clip_path(Rect::from_origin_size(Point::ORIGIN, size));
        } else {
            ctx.clear_clip_path();
        }

        let baseline = ctx.child_baseline_offset(&self.child);
        ctx.set_baseline_offset(baseline);
    }

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
        trace_span!("Clip", id = id.trace())
    }

    fn get_debug_text(&self) -> Option<String> {
        self.enabled.then_some("enabled".into())
    }
}

// --- MARK: TESTS
#[cfg(test)]
mod tests {
    use super::*;
    use crate::kurbo::Size;
    use crate::testing::TestHarness;
    use crate::theme::test_property_set;
    use crate::widgets::Label;

    #[test]
    fn sets_clip_path_when_enabled() {
        let widget = Clip::new(Label::new("Hello").with_auto_id()).with_auto_id();
        let harness =
            TestHarness::create_with_size(test_property_set(), widget, Size::new(50.0, 20.0));

        let root = harness.root_widget();
        assert!(root.ctx().clip_path().is_some());
    }

    #[test]
    fn clears_clip_path_when_disabled() {
        let widget = Clip::new(Label::new("Hello").with_auto_id())
            .enabled(false)
            .with_auto_id();
        let harness =
            TestHarness::create_with_size(test_property_set(), widget, Size::new(50.0, 20.0));

        let root = harness.root_widget();
        assert!(root.ctx().clip_path().is_none());
    }

    #[test]
    fn can_toggle_enabled_at_runtime() {
        let widget = Clip::new(Label::new("Hello").with_auto_id()).with_auto_id();
        let mut harness =
            TestHarness::create_with_size(test_property_set(), widget, Size::new(50.0, 20.0));

        assert!(harness.root_widget().ctx().clip_path().is_some());

        harness.edit_root_widget(|mut clip| {
            Clip::set_enabled(&mut clip, false);
        });
        assert!(harness.root_widget().ctx().clip_path().is_none());

        harness.edit_root_widget(|mut clip| {
            Clip::set_enabled(&mut clip, true);
        });
        assert!(harness.root_widget().ctx().clip_path().is_some());
    }
}
