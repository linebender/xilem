// Copyright 2018 the Xilem Authors and the Druid Authors
// SPDX-License-Identifier: Apache-2.0

//! A widget that aligns its child (for example, centering it).

// TODO - Improve the ergonomics of widget layout. The Align widget is a bandaid
// that has several problem; in particular, the fact that Align will pass "loosened"
// size constraints to its child means that "aligning" a widget may actually change
// its computed size. See https://github.com/linebender/xilem/issues/378

use accesskit::{NodeBuilder, Role};
use smallvec::{smallvec, SmallVec};
use tracing::{trace, trace_span, Span};
use vello::Scene;

use crate::contexts::AccessCtx;
use crate::paint_scene_helpers::UnitPoint;
use crate::widget::WidgetPod;
use crate::{
    AccessEvent, BoxConstraints, EventCtx, LayoutCtx, LifeCycleCtx, PaintCtx, PointerEvent, Rect,
    RegisterCtx, Size, StatusChange, TextEvent, Widget, WidgetId,
};

// TODO - Have child widget type as generic argument

/// A widget that aligns its child.
pub struct Align {
    align: UnitPoint,
    child: WidgetPod<Box<dyn Widget>>,
    width_factor: Option<f64>,
    height_factor: Option<f64>,
}

// --- MARK: BUILDERS ---
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

// --- MARK: IMPL WIDGET ---
impl Widget for Align {
    fn on_pointer_event(&mut self, _ctx: &mut EventCtx, _event: &PointerEvent) {}

    fn on_text_event(&mut self, _ctx: &mut EventCtx, _event: &TextEvent) {}

    fn on_access_event(&mut self, _ctx: &mut EventCtx, _event: &AccessEvent) {}

    fn register_children(&mut self, ctx: &mut RegisterCtx) {
        ctx.register_child(&mut self.child);
    }

    fn on_status_change(&mut self, _ctx: &mut LifeCycleCtx, _event: &StatusChange) {}

    fn layout(&mut self, ctx: &mut LayoutCtx, bc: &BoxConstraints) -> Size {
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
            .resolve(Rect::new(0., 0., extra_width, extra_height))
            .expand();
        ctx.place_child(&mut self.child, origin);

        let my_insets = ctx.compute_insets_from_child(&self.child, my_size);
        ctx.set_paint_insets(my_insets);
        if self.height_factor.is_some() {
            let baseline_offset = ctx.child_baseline_offset(&self.child);
            if baseline_offset > 0_f64 {
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

    fn paint(&mut self, _ctx: &mut PaintCtx, _scene: &mut Scene) {}

    fn accessibility_role(&self) -> Role {
        Role::GenericContainer
    }

    fn accessibility(&mut self, _ctx: &mut AccessCtx, _node: &mut NodeBuilder) {}

    fn children_ids(&self) -> SmallVec<[WidgetId; 16]> {
        smallvec![self.child.id()]
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

// --- MARK: TESTS ---
// --- MARK: TESTS ---
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
