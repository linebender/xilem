#![allow(unused)]

use std::ops::Range;

use crate::kurbo::{Affine, Point, Rect, Size, Vec2};
use crate::widget::prelude::*;
use crate::widget::widget_view::WidgetRef;
use crate::widget::widget_view::WidgetView;
use crate::widget::Axis;
use crate::WidgetPod;
use druid_shell::kurbo::Shape;
use smallvec::{smallvec, SmallVec};
use tracing::{trace, trace_span, warn, Span};

// TODO - rename ScrollPortal?
// Conceptually, a Portal is a Widget giving a restricted view of a child widget
// Imagine a very large widget, and a rect that represents the part of the widget we see
pub struct Portal<W: Widget> {
    child: WidgetPod<W>,
    // TODO - differentiate between the "explicit" viewport pos determined
    // by user input, and the computed viewport pos that may change based
    // on re-layouts
    // TODO - rename
    viewport_pos: Point,
    // TODO - test how it looks like
    constrain_horizontal: bool,
    constrain_vertical: bool,
    must_fill: bool,
}

impl<W: Widget> Portal<W> {
    pub fn new(child: W) -> Self {
        Portal {
            child: WidgetPod::new(child),
            viewport_pos: Point::ORIGIN,
            constrain_horizontal: false,
            constrain_vertical: false,
            must_fill: false,
        }
    }

    pub fn get_viewport_pos(&self) -> Point {
        self.viewport_pos
    }

    // TODO - rewrite doc
    /// Builder-style method for deciding whether to constrain the child vertically.
    ///
    /// The default is `false`.
    ///
    /// This setting affects how a `ClipBox` lays out its child.
    ///
    /// - When it is `false` (the default), the child does not receive any upper
    ///   bound on its height: the idea is that the child can be as tall as it
    ///   wants, and the viewport will somehow get moved around to see all of it.
    /// - When it is `true`, the viewport's maximum height will be passed down
    ///   as an upper bound on the height of the child, and the viewport will set
    ///   its own height to be the same as its child's height.
    pub fn constrain_vertical(mut self, constrain: bool) -> Self {
        self.constrain_vertical = constrain;
        self
    }

    /// Builder-style method for deciding whether to constrain the child horizontally.
    ///
    /// The default is `false`. See [`constrain_vertical`] for more details.
    ///
    /// [`constrain_vertical`]: struct.ClipBox.html#constrain_vertical
    pub fn constrain_horizontal(mut self, constrain: bool) -> Self {
        self.constrain_horizontal = constrain;
        self
    }

    /// Builder-style method to set whether the child must fill the view.
    ///
    /// If `false` (the default) there is no minimum constraint on the child's
    /// size. If `true`, the child is passed the same minimum constraints as
    /// the `ClipBox`.
    pub fn content_must_fill(mut self, must_fill: bool) -> Self {
        self.must_fill = must_fill;
        self
    }
}

fn compute_pan_range(mut viewport: Range<f64>, target: Range<f64>) -> Range<f64> {
    // if either range contains the other, the viewport doesn't move
    if target.start <= viewport.start && viewport.end <= target.end {
        return viewport;
    }
    if viewport.start <= target.start && target.end <= viewport.end {
        return viewport;
    }

    // we compute the length that we need to "fit" in our viewport
    let mut target_width = f64::min(viewport.end - viewport.start, target.end - target.start);
    let mut viewport_width = viewport.end - viewport.start;

    // Because of the early returns, there are only two cases to consider: we need
    // to move the viewport "left" or "right"
    if viewport.start >= target.start {
        viewport.start = target.end - target_width;
        viewport.end = viewport.start + viewport_width;
    } else {
        viewport.end = target.start + target_width;
        viewport.start = viewport.end - viewport_width;
    }

    viewport
}

impl<W: Widget> Portal<W> {
    // TODO - rename
    fn set_viewport_pos_raw(&mut self, content_size: Size, portal_size: Size, pos: Point) -> bool {
        let viewport_max_pos =
            (content_size - portal_size).clamp(Size::ZERO, Size::new(f64::INFINITY, f64::INFINITY));
        let pos = Point::new(
            pos.x.clamp(0.0, viewport_max_pos.width),
            pos.y.clamp(0.0, viewport_max_pos.height),
        );

        if (pos - self.viewport_pos).hypot2() > 1e-12 {
            self.viewport_pos = pos;
            true
        } else {
            false
        }
    }
}

impl<'a, 'b, W: Widget> WidgetView<'a, 'b, Portal<W>> {
    pub fn get_child_view(&mut self) -> WidgetView<'_, 'b, W> {
        let child = &mut self.widget.child;
        WidgetView {
            global_state: self.global_state,
            parent_widget_state: self.widget_state,
            widget_state: &mut child.state,
            widget: &mut child.inner,
        }
    }

    // TODO - rewrite doc
    /// Set whether to constrain the child horizontally.
    ///
    /// See [`constrain_vertical`] for more details.
    ///
    /// [`constrain_vertical`]: struct.ClipBox.html#constrain_vertical
    pub fn set_constrain_horizontal(&mut self, constrain: bool) {
        self.widget.constrain_horizontal = constrain;
        self.request_layout();
    }

    /// Set whether to constrain the child vertically.
    ///
    /// See [`constrain_vertical`] for more details.
    ///
    /// [`constrain_vertical`]: struct.ClipBox.html#constrain_vertical
    pub fn set_constrain_vertical(&mut self, constrain: bool) {
        self.widget.constrain_vertical = constrain;
        self.request_layout();
    }

    /// Set whether the child's size must be greater than or equal the size of
    /// the `ClipBox`.
    ///
    /// See [`content_must_fill`] for more details.
    ///
    /// [`content_must_fill`]: ClipBox::content_must_fill
    pub fn set_content_must_fill(&mut self, must_fill: bool) {
        self.widget.must_fill = must_fill;
        self.request_layout();
    }

    pub fn set_viewport_pos(&mut self, position: Point) -> bool {
        let pos_changed = self.widget.set_viewport_pos_raw(
            self.widget.child.layout_rect().size(),
            self.widget_state.layout_rect().size(),
            position,
        );
        // TODO
        if true || pos_changed {
            self.request_layout();
        }
        pos_changed
    }

    pub fn pan_viewport_by(&mut self, translation: Vec2) -> bool {
        self.set_viewport_pos(self.widget.viewport_pos + translation)
    }

    // Note - Rect is in child coordinates
    pub fn pan_viewport_to(&mut self, target: Rect) -> bool {
        let viewport = Rect::from_origin_size(self.widget.viewport_pos, self.widget_state.size);

        let new_pos_x = compute_pan_range(
            viewport.min_x()..viewport.max_x(),
            target.min_x()..target.max_x(),
        )
        .start;
        let new_pos_y = compute_pan_range(
            viewport.min_y()..viewport.max_y(),
            target.min_y()..target.max_y(),
        )
        .start;

        self.set_viewport_pos(Point::new(new_pos_x, new_pos_y))
    }
}

impl<W: Widget> Widget for Portal<W> {
    fn on_event(&mut self, ctx: &mut EventCtx, event: &Event, env: &Env) {
        ctx.init();

        // TODO - move to scroll widget
        if let Event::Wheel(wheel_event) = event {
            self.viewport_pos += wheel_event.wheel_delta;
        }

        self.child.on_event(ctx, &event, env);
        ctx.request_layout();
    }

    fn on_status_change(&mut self, _ctx: &mut LifeCycleCtx, _event: &StatusChange, _env: &Env) {}

    fn lifecycle(&mut self, ctx: &mut LifeCycleCtx, event: &LifeCycle, env: &Env) {
        ctx.init();

        self.child.lifecycle(ctx, event, env);

        /*
        match event {
            LifeCycle::RequestPanToChild(target_rect) => {}
            _ => {}
        }
        */
    }

    fn layout(&mut self, ctx: &mut LayoutCtx, bc: &BoxConstraints, env: &Env) -> Size {
        ctx.init();

        let min_child_size = if self.must_fill { bc.min() } else { Size::ZERO };
        let mut max_child_size = bc.max();
        if !self.constrain_horizontal {
            max_child_size.width = f64::INFINITY
        };
        if !self.constrain_vertical {
            max_child_size.height = f64::INFINITY
        };

        let child_bc = BoxConstraints::new(min_child_size, max_child_size);

        let content_size = self.child.layout(ctx, &child_bc, env);
        let portal_size = bc.constrain(content_size);

        // TODO - document better
        // Recompute the portal offset for the new layout
        self.set_viewport_pos_raw(content_size, portal_size, self.viewport_pos);

        self.child
            .set_origin(ctx, env, Point::new(0.0, -self.viewport_pos.y));

        portal_size
    }

    fn paint(&mut self, ctx: &mut PaintCtx, env: &Env) {
        ctx.init();

        // TODO - have ctx.clip also clip the invalidated region
        let clip_rect = ctx.size().to_rect();
        ctx.clip(clip_rect);

        self.child.paint(ctx, env);
    }

    fn children(&self) -> SmallVec<[WidgetRef<'_, dyn Widget>; 16]> {
        smallvec![self.child.as_dyn()]
    }

    fn make_trace_span(&self) -> Span {
        trace_span!("Portal")
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::assert_render_snapshot;
    use crate::testing::{widget_ids, Harness};
    use crate::theme::{PRIMARY_DARK, PRIMARY_LIGHT};
    use crate::widget::{Button, Flex, SizedBox};
    use insta::assert_debug_snapshot;
    use piet_common::FontFamily;

    fn button(text: &str) -> impl Widget {
        SizedBox::new(Button::new(text)).width(70.0).height(40.0)
    }

    #[test]
    fn button_list() {
        let [item_3_id, item_13_id] = widget_ids();

        let widget = Portal::new(
            Flex::column()
                .with_child(button("Item 1"))
                .with_spacer(10.0)
                .with_child(button("Item 2"))
                .with_spacer(10.0)
                .with_child_id(button("Item 3"), item_3_id)
                .with_spacer(10.0)
                .with_child(button("Item 4"))
                .with_spacer(10.0)
                .with_child(button("Item 5"))
                .with_spacer(10.0)
                .with_child(button("Item 6"))
                .with_spacer(10.0)
                .with_child(button("Item 7"))
                .with_spacer(10.0)
                .with_child(button("Item 8"))
                .with_spacer(10.0)
                .with_child(button("Item 9"))
                .with_spacer(10.0)
                .with_child(button("Item 10"))
                .with_spacer(10.0)
                .with_child(button("Item 11"))
                .with_spacer(10.0)
                .with_child(button("Item 12"))
                .with_spacer(10.0)
                .with_child_id(button("Item 13"), item_13_id)
                .with_spacer(10.0)
                .with_child(button("Item 14"))
                .with_spacer(10.0),
        );

        let mut harness = Harness::create_with_size(widget, Size::new(400., 400.));

        assert_debug_snapshot!(harness.root_widget());
        assert_render_snapshot!(harness, "button_list_no_scroll");

        harness.edit_root_widget(|mut portal, _| {
            let mut portal = portal.downcast::<Portal<Flex>>().unwrap();
            portal.set_viewport_pos(Point::new(0.0, 130.0))
        });

        assert_render_snapshot!(harness, "button_list_scrolled");

        let item_3_rect = harness.get_widget(item_3_id).state().layout_rect();
        harness.edit_root_widget(|mut portal, _| {
            let mut portal = portal.downcast::<Portal<Flex>>().unwrap();
            portal.pan_viewport_to(item_3_rect);
        });

        assert_render_snapshot!(harness, "button_list_scroll_to_item_3");

        let item_13_rect = harness.get_widget(item_13_id).state().layout_rect();
        harness.edit_root_widget(|mut portal, _| {
            let mut portal = portal.downcast::<Portal<Flex>>().unwrap();
            portal.pan_viewport_to(item_13_rect);
        });

        assert_render_snapshot!(harness, "button_list_scroll_to_item_13");
    }

    // Helper function for panning tests
    fn make_range(repr: &str) -> Range<f64> {
        let repr = &repr[repr.find('_').unwrap()..];

        let start = repr.find('x').unwrap();
        let end = repr[start..].find('_').unwrap() + start;

        assert!(repr[end..].chars().all(|c| c == '_'));

        (start as f64)..(end as f64)
    }

    #[test]
    fn test_pan_to_same() {
        let initial_range = make_range("_______xxxx_____");
        let target_range = make_range(" _______xxxx_____");
        let result_range = make_range(" _______xxxx_____");

        assert_eq!(compute_pan_range(initial_range, target_range), result_range);
    }

    #[test]
    fn test_pan_to_smaller() {
        let initial_range = make_range("_____xxxxxxxx___");
        let target_range = make_range(" _______xxxx_____");
        let result_range = make_range(" _____xxxxxxxx___");

        assert_eq!(compute_pan_range(initial_range, target_range), result_range);
    }

    #[test]
    fn test_pan_to_larger() {
        let initial_range = make_range("_______xxxx_____");
        let target_range = make_range(" _____xxxxxxxx___");
        let result_range = make_range(" _______xxxx_____");

        assert_eq!(compute_pan_range(initial_range, target_range), result_range);
    }

    #[test]
    fn test_pan_to_left() {
        let initial_range = make_range("_______xxxx_____");
        let target_range = make_range(" ____xx__________");
        let result_range = make_range(" ____xxxx________");

        assert_eq!(compute_pan_range(initial_range, target_range), result_range);
    }

    #[test]
    fn test_pan_to_left_intersects() {
        let initial_range = make_range("_______xxxxx____");
        let target_range = make_range(" ____xxxx________");
        let result_range = make_range(" ____xxxxx_______");

        assert_eq!(compute_pan_range(initial_range, target_range), result_range);
    }

    #[test]
    fn test_pan_to_left_larger() {
        let initial_range = make_range("__________xx____");
        let target_range = make_range(" ____xxxx________");
        let result_range = make_range(" ______xx________");

        assert_eq!(compute_pan_range(initial_range, target_range), result_range);
    }

    #[test]
    fn test_pan_to_left_larger_intersects() {
        let initial_range = make_range("_______xx_______");
        let target_range = make_range(" ____xxxx________");
        let result_range = make_range(" ______xx________");

        assert_eq!(compute_pan_range(initial_range, target_range), result_range);
    }

    #[test]
    fn test_pan_to_right() {
        let initial_range = make_range("_____xxxx_______");
        let target_range = make_range(" __________xx____");
        let result_range = make_range(" ________xxxx____");

        assert_eq!(compute_pan_range(initial_range, target_range), result_range);
    }

    #[test]
    fn test_pan_to_right_intersects() {
        let initial_range = make_range("____xxxxx_______");
        let target_range = make_range(" ________xxxx____");
        let result_range = make_range(" _______xxxxx____");

        assert_eq!(compute_pan_range(initial_range, target_range), result_range);
    }

    #[test]
    fn test_pan_to_right_larger() {
        let initial_range = make_range("____xx__________");
        let target_range = make_range(" ________xxxx____");
        let result_range = make_range(" ________xx______");

        assert_eq!(compute_pan_range(initial_range, target_range), result_range);
    }

    #[test]
    fn test_pan_to_right_larger_intersects() {
        let initial_range = make_range("_______xx_______");
        let target_range = make_range(" ________xxxx____");
        let result_range = make_range(" ________xx______");

        assert_eq!(compute_pan_range(initial_range, target_range), result_range);
    }
}
