// Copyright 2018 the Xilem Authors and the Druid Authors
// SPDX-License-Identifier: Apache-2.0

//! A widget that aligns its child (for example, centering it).

// TODO - Improve the ergonomics of widget layout. The Align widget is a bandaid
// that has several problem; in particular, the fact that Align will pass "loosened"
// size constraints to its child means that "aligning" a widget may actually change
// its computed size. See https://github.com/linebender/xilem/issues/378

use accesskit::{Node, Role};
use masonry_core::core::WidgetMut;
use tracing::{Span, trace_span};
use vello::Scene;
use vello::kurbo::{Rect, Size};

use crate::core::{
    AccessCtx, BoxConstraints, ChildrenIds, LayoutCtx, NewWidget, NoAction, PaintCtx,
    PropertiesMut, PropertiesRef, RegisterCtx, Widget, WidgetId, WidgetPod,
};
use crate::properties::types::UnitPoint;
use crate::util::include_screenshot;

// TODO - Have child widget type as generic argument

/// A widget that aligns its child.
///
#[doc = include_screenshot!("align_right.png", "Right-aligned label.")]
pub struct Align {
    align: UnitPoint,
    child: WidgetPod<dyn Widget>,
    width_factor: Option<f64>,
    height_factor: Option<f64>,
}

// --- MARK: BUILDERS
impl Align {
    /// Create widget with alignment.
    ///
    /// Note that the `align` parameter is specified as a `UnitPoint` in
    /// terms of left and right. This is inadequate for bidi-aware layout
    /// and thus the API will change when Masonry gains bidi capability.
    pub fn new(align: UnitPoint, child: NewWidget<impl Widget + ?Sized>) -> Self {
        Self {
            align,
            child: child.erased().to_pod(),
            width_factor: None,
            height_factor: None,
        }
    }

    /// Create centered widget.
    pub fn centered(child: NewWidget<impl Widget + ?Sized>) -> Self {
        Self::new(UnitPoint::CENTER, child)
    }

    /// Create right-aligned widget.
    pub fn right(child: NewWidget<impl Widget + ?Sized>) -> Self {
        Self::new(UnitPoint::RIGHT, child)
    }

    /// Create left-aligned widget.
    pub fn left(child: NewWidget<impl Widget + ?Sized>) -> Self {
        Self::new(UnitPoint::LEFT, child)
    }

    /// Align only in the horizontal axis, keeping the child's size in the vertical.
    pub fn horizontal(align: UnitPoint, child: NewWidget<impl Widget + ?Sized>) -> Self {
        Self {
            align,
            child: child.erased().to_pod(),
            width_factor: None,
            height_factor: Some(1.0),
        }
    }

    /// Align only in the vertical axis, keeping the child's size in the horizontal.
    pub fn vertical(align: UnitPoint, child: NewWidget<impl Widget + ?Sized>) -> Self {
        Self {
            align,
            child: child.erased().to_pod(),
            width_factor: Some(1.0),
            height_factor: None,
        }
    }
}

// --- MARK: WIDGETMUT
impl Align {
    /// Replace the child widget with a new one.
    pub fn set_child(this: &mut WidgetMut<'_, Self>, child: NewWidget<impl Widget + ?Sized>) {
        this.ctx.remove_child(std::mem::replace(
            &mut this.widget.child,
            child.erased().to_pod(),
        ));
    }

    /// Get mutable reference to the child widget.
    pub fn child_mut<'t>(this: &'t mut WidgetMut<'_, Self>) -> WidgetMut<'t, dyn Widget> {
        this.ctx.get_mut(&mut this.widget.child)
    }
}

// --- MARK: IMPL WIDGET
impl Widget for Align {
    type Action = NoAction;

    fn register_children(&mut self, ctx: &mut RegisterCtx<'_>) {
        ctx.register_child(&mut self.child);
    }

    fn layout(
        &mut self,
        ctx: &mut LayoutCtx<'_>,
        _props: &mut PropertiesMut<'_>,
        bc: &BoxConstraints,
    ) -> Size {
        let size = ctx.run_layout(&mut self.child, &bc.loosen());

        log_size_warnings(size);

        let mut my_size = size;
        if bc.is_width_bounded() {
            my_size.width = bc.max().width;
        }
        if bc.is_height_bounded() {
            my_size.height = bc.max().height;
        }

        if let Some(width) = self.width_factor {
            my_size.width = size.width * width;
        }
        if let Some(height) = self.height_factor {
            my_size.height = size.height * height;
        }

        my_size = bc.constrain(my_size);
        let extra_width = (my_size.width - size.width).max(0.);
        let extra_height = (my_size.height - size.height).max(0.);
        let origin = self
            .align
            .resolve(Rect::new(0., 0., extra_width, extra_height));
        ctx.place_child(&mut self.child, origin);

        let my_insets = ctx.compute_insets_from_child(&self.child, my_size);
        ctx.set_paint_insets(my_insets);
        if self.height_factor.is_some() {
            let baseline_offset = ctx.child_baseline_offset(&self.child);
            if baseline_offset > 0_f64 {
                ctx.set_baseline_offset(baseline_offset + extra_height / 2.0);
            }
        }

        my_size
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
        trace_span!("Align", id = id.trace())
    }
}

fn log_size_warnings(size: Size) {
    if size.width.is_infinite() {
        tracing::warn!("Align widget's child has an infinite width.");
    }

    if size.height.is_infinite() {
        tracing::warn!("Align widget's child has an infinite height.");
    }
}

// --- MARK: TESTS
#[cfg(test)]
mod tests {
    use super::*;
    use crate::testing::{TestHarness, assert_render_snapshot};
    use crate::theme::test_property_set;
    use crate::widgets::Label;

    // TODO - Add more unit tests

    #[test]
    fn centered() {
        let widget = Align::centered(Label::new("hello").with_auto_id()).with_auto_id();

        let mut harness = TestHarness::create(test_property_set(), widget);

        assert_render_snapshot!(harness, "align_centered");
    }

    #[test]
    fn right() {
        let widget = Align::right(Label::new("hello").with_auto_id()).with_auto_id();

        let mut harness = TestHarness::create(test_property_set(), widget);

        assert_render_snapshot!(harness, "align_right");
    }

    #[test]
    fn left() {
        let widget = Align::left(Label::new("hello").with_auto_id()).with_auto_id();

        let mut harness = TestHarness::create(test_property_set(), widget);

        assert_render_snapshot!(harness, "align_left");
    }
}
