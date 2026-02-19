// Copyright 2026 the Xilem Authors
// SPDX-License-Identifier: Apache-2.0

use accesskit::{Node, Role};
use tracing::{Span, trace_span};
use vello::Scene;
use vello::kurbo::{Point, Size};

use crate::core::{
    AccessCtx, ChildrenIds, LayoutCtx, MeasureCtx, NewWidget, NoAction, PaintCtx, PropertiesRef,
    RegisterCtx, Widget, WidgetId, WidgetMut, WidgetPod,
};
use crate::kurbo::Axis;
use crate::layout::LenReq;

/// A radio group container that holds radio buttons.
pub struct RadioGroup {
    pub(crate) child: WidgetPod<dyn Widget>,
    pub(crate) selected_button: Option<WidgetId>,
}

impl RadioGroup {
    /// Create a new `RadioGroup`.
    pub fn new(child: NewWidget<impl Widget + ?Sized>) -> Self {
        Self {
            child: child.erased().to_pod(),
            selected_button: None,
        }
    }
}

// --- MARK: WIDGETMUT
impl RadioGroup {
    /// Get mutable reference to the child widget.
    pub fn child_mut<'t>(this: &'t mut WidgetMut<'_, Self>) -> WidgetMut<'t, dyn Widget> {
        let child = &mut this.widget.child;
        this.ctx.get_mut(child)
    }
}

// --- MARK: IMPL WIDGET
impl Widget for RadioGroup {
    type Action = NoAction;

    // TODO: navigation shortcuts.

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

        let child_baseline = ctx.child_baseline_offset(&self.child);
        ctx.set_baseline_offset(child_baseline);
    }

    fn paint(&mut self, _ctx: &mut PaintCtx<'_>, _props: &PropertiesRef<'_>, _scene: &mut Scene) {}

    fn accessibility_role(&self) -> Role {
        Role::RadioGroup
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
        trace_span!("RadioGroup", id = id.trace())
    }
}

// TODO: make tests.
