// This software is licensed under Apache License 2.0 and distributed on an
// "as-is" basis without warranties of any kind. See the LICENSE file for
// details.

//! A widget that aligns its child (for example, centering it).

// TODO - Improve the ergonomics of widget layout. The Align widget is a bandaid
// that has several problem; in particular, the fact that Align will pass "loosened"
// size constraints to its child means that "aligning" a widget may actually change
// its computed size. See issue #3.

use smallvec::{smallvec, SmallVec};
use tracing::{trace, trace_span, Span};

use crate::widget::{WidgetPod, WidgetRef};
use crate::{
    BoxConstraints, Event, EventCtx, LayoutCtx, LifeCycle, LifeCycleCtx, PaintCtx, Rect, Size,
    StatusChange, UnitPoint, Widget,
};

// TODO - Have child widget type as generic argument

/// A widget that aligns its child.
pub struct Align {
    align: UnitPoint,
    child: WidgetPod<Box<dyn Widget>>,
    width_factor: Option<f64>,
    height_factor: Option<f64>,
}

impl Align {
    /// Create widget with alignment.
    ///
    /// Note that the `align` parameter is specified as a `UnitPoint` in
    /// terms of left and right. This is inadequate for bidi-aware layout
    /// and thus the API will change when Masonry gains bidi capability.
    pub fn new(align: UnitPoint, child: impl Widget + 'static) -> Align {
        Align {
            align,
            child: WidgetPod::new(child).boxed(),
            width_factor: None,
            height_factor: None,
        }
    }

    /// Create centered widget.
    pub fn centered(child: impl Widget + 'static) -> Align {
        Align::new(UnitPoint::CENTER, child)
    }

    /// Create right-aligned widget.
    pub fn right(child: impl Widget + 'static) -> Align {
        Align::new(UnitPoint::RIGHT, child)
    }

    /// Create left-aligned widget.
    pub fn left(child: impl Widget + 'static) -> Align {
        Align::new(UnitPoint::LEFT, child)
    }

    /// Align only in the horizontal axis, keeping the child's size in the vertical.
    pub fn horizontal(align: UnitPoint, child: impl Widget + 'static) -> Align {
        Align {
            align,
            child: WidgetPod::new(child).boxed(),
            width_factor: None,
            height_factor: Some(1.0),
        }
    }

    /// Align only in the vertical axis, keeping the child's size in the horizontal.
    pub fn vertical(align: UnitPoint, child: impl Widget + 'static) -> Align {
        Align {
            align,
            child: WidgetPod::new(child).boxed(),
            width_factor: Some(1.0),
            height_factor: None,
        }
    }
}

impl Widget for Align {
    fn on_event(&mut self, ctx: &mut EventCtx, event: &Event) {
        self.child.on_event(ctx, event)
    }

    fn lifecycle(&mut self, ctx: &mut LifeCycleCtx, event: &LifeCycle) {
        self.child.lifecycle(ctx, event)
    }

    fn on_status_change(&mut self, _ctx: &mut LifeCycleCtx, _event: &StatusChange) {}

    fn layout(&mut self, ctx: &mut LayoutCtx, bc: &BoxConstraints) -> Size {
        let size = self.child.layout(ctx, &bc.loosen());

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
            .resolve(Rect::new(0., 0., extra_width, extra_height))
            .expand();
        ctx.place_child(&mut self.child, origin);

        let my_insets = self.child.compute_parent_paint_insets(my_size);
        ctx.set_paint_insets(my_insets);
        if self.height_factor.is_some() {
            let baseline_offset = self.child.baseline_offset();
            if baseline_offset > 0f64 {
                ctx.set_baseline_offset(baseline_offset + extra_height / 2.0);
            }
        }

        trace!(
            "Computed layout: origin={}, size={}, insets={:?}",
            origin,
            my_size,
            my_insets
        );
        my_size
    }

    fn paint(&mut self, ctx: &mut PaintCtx) {
        self.child.paint(ctx);
    }

    fn children(&self) -> SmallVec<[WidgetRef<'_, dyn Widget>; 16]> {
        smallvec![self.child.as_dyn()]
    }

    fn make_trace_span(&self) -> Span {
        trace_span!("Align")
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

#[cfg(test)]
mod tests {
    use insta::assert_debug_snapshot;

    use super::*;
    use crate::assert_render_snapshot;
    use crate::testing::TestHarness;
    use crate::widget::Label;

    // TODO - Add more unit tests

    #[test]
    fn centered() {
        let widget = Align::centered(Label::new("hello"));

        let mut harness = TestHarness::create(widget);

        assert_debug_snapshot!(harness.root_widget());
        assert_render_snapshot!(harness, "centered");
    }

    #[test]
    fn right() {
        let widget = Align::right(Label::new("hello"));

        let mut harness = TestHarness::create(widget);

        assert_debug_snapshot!(harness.root_widget());
        assert_render_snapshot!(harness, "right");
    }

    #[test]
    fn left() {
        let widget = Align::left(Label::new("hello"));

        let mut harness = TestHarness::create(widget);

        assert_debug_snapshot!(harness.root_widget());
        assert_render_snapshot!(harness, "left");
    }
}
