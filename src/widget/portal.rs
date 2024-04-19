// This software is licensed under Apache License 2.0 and distributed on an
// "as-is" basis without warranties of any kind. See the LICENSE file for
// details.

#![allow(missing_docs)]

use std::ops::Range;

use kurbo::Affine;
use smallvec::{smallvec, SmallVec};
use tracing::{trace_span, Span};
use vello::peniko::BlendMode;
use vello::Scene;

use crate::kurbo::{Point, Rect, Size, Vec2};
use crate::widget::{Axis, ScrollBar, StoreInWidgetMut, WidgetMut, WidgetRef};
use crate::{
    BoxConstraints, EventCtx, LayoutCtx, LifeCycle, LifeCycleCtx, PaintCtx, PointerEvent,
    StatusChange, TextEvent, Widget, WidgetPod,
};

// TODO - refactor - see issue #15
// TODO - rename "Portal" to "ScrollPortal"?
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
    scrollbar_horizontal: WidgetPod<ScrollBar>,
    scrollbar_horizontal_visible: bool,
    scrollbar_vertical: WidgetPod<ScrollBar>,
    scrollbar_vertical_visible: bool,
}

crate::declare_widget!(PortalMut, Portal<W: (Widget)>);

impl<W: Widget> Portal<W> {
    pub fn new(child: W) -> Self {
        Portal {
            child: WidgetPod::new(child),
            viewport_pos: Point::ORIGIN,
            constrain_horizontal: false,
            constrain_vertical: false,
            must_fill: false,
            // TODO - remove
            scrollbar_horizontal: WidgetPod::new(ScrollBar::new(Axis::Horizontal, 1.0, 1.0)),
            scrollbar_horizontal_visible: false,
            scrollbar_vertical: WidgetPod::new(ScrollBar::new(Axis::Vertical, 1.0, 1.0)),
            scrollbar_vertical_visible: false,
        }
    }

    pub fn get_viewport_pos(&self) -> Point {
        self.viewport_pos
    }

    pub fn child(&self) -> WidgetRef<'_, W> {
        self.child.as_ref()
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
    let target_width = f64::min(viewport.end - viewport.start, target.end - target.start);
    let viewport_width = viewport.end - viewport.start;

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
    fn set_viewport_pos_raw(&mut self, portal_size: Size, content_size: Size, pos: Point) -> bool {
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

impl<'a, W: Widget> PortalMut<'a, W> {
    pub fn child_mut(&mut self) -> WidgetMut<'_, W>
    where
        W: StoreInWidgetMut,
    {
        self.ctx.get_mut(&mut self.widget.child)
    }

    pub fn horizontal_scrollbar_mut(&mut self) -> WidgetMut<'_, ScrollBar> {
        self.ctx.get_mut(&mut self.widget.scrollbar_horizontal)
    }

    pub fn vertical_scrollbar_mut(&mut self) -> WidgetMut<'_, ScrollBar> {
        self.ctx.get_mut(&mut self.widget.scrollbar_vertical)
    }

    // TODO - rewrite doc
    /// Set whether to constrain the child horizontally.
    ///
    /// See [`constrain_vertical`] for more details.
    ///
    /// [`constrain_vertical`]: struct.ClipBox.html#constrain_vertical
    pub fn set_constrain_horizontal(&mut self, constrain: bool) {
        self.widget.constrain_horizontal = constrain;
        self.ctx.request_layout();
    }

    /// Set whether to constrain the child vertically.
    ///
    /// See [`constrain_vertical`] for more details.
    ///
    /// [`constrain_vertical`]: struct.ClipBox.html#constrain_vertical
    pub fn set_constrain_vertical(&mut self, constrain: bool) {
        self.widget.constrain_vertical = constrain;
        self.ctx.request_layout();
    }

    /// Set whether the child's size must be greater than or equal the size of
    /// the `ClipBox`.
    ///
    /// See [`content_must_fill`] for more details.
    ///
    /// [`content_must_fill`]: ClipBox::content_must_fill
    pub fn set_content_must_fill(&mut self, must_fill: bool) {
        self.widget.must_fill = must_fill;
        self.ctx.request_layout();
    }

    pub fn set_viewport_pos(&mut self, position: Point) -> bool {
        let portal_size = self.ctx.widget_state.layout_rect().size();
        let content_size = self.widget.child.layout_rect().size();

        let pos_changed = self
            .widget
            .set_viewport_pos_raw(portal_size, content_size, position);
        if pos_changed {
            let progress_x = self.widget.viewport_pos.x / (content_size - portal_size).width;
            self.horizontal_scrollbar_mut()
                .set_cursor_progress(progress_x);
            let progress_y = self.widget.viewport_pos.y / (content_size - portal_size).height;
            self.vertical_scrollbar_mut()
                .set_cursor_progress(progress_y);
            self.ctx.request_layout();
        }
        pos_changed
    }

    pub fn pan_viewport_by(&mut self, translation: Vec2) -> bool {
        self.set_viewport_pos(self.widget.viewport_pos + translation)
    }

    // Note - Rect is in child coordinates
    pub fn pan_viewport_to(&mut self, target: Rect) -> bool {
        let viewport = Rect::from_origin_size(self.widget.viewport_pos, self.ctx.widget_state.size);

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
    fn on_pointer_event(&mut self, ctx: &mut EventCtx, event: &PointerEvent) {
        let portal_size = ctx.size();
        let content_size = self.child.layout_rect().size();

        match event {
            PointerEvent::MouseWheel(delta, _) => {
                self.set_viewport_pos_raw(
                    portal_size,
                    content_size,
                    self.viewport_pos + Vec2::new(delta.x, delta.y),
                );
                // TODO - horizontal scrolling?
                ctx.get_mut(&mut self.scrollbar_vertical)
                    .set_cursor_progress(self.viewport_pos.y / (content_size - portal_size).height);
                ctx.request_layout();
            }
            _ => (),
        }

        self.child.on_pointer_event(ctx, event);
        self.scrollbar_horizontal.on_pointer_event(ctx, event);
        self.scrollbar_vertical.on_pointer_event(ctx, event);

        if self.scrollbar_horizontal.widget().moved {
            let progress = self.scrollbar_horizontal.widget().cursor_progress;
            self.scrollbar_horizontal.widget_mut().moved = false;
            self.viewport_pos = Axis::Horizontal
                .pack(
                    progress * Axis::Horizontal.major(content_size - portal_size),
                    Axis::Horizontal.minor_pos(self.viewport_pos),
                )
                .into();
            ctx.request_layout();
        }
        if self.scrollbar_vertical.widget().moved {
            let progress = self.scrollbar_vertical.widget().cursor_progress;
            self.scrollbar_vertical.widget_mut().moved = false;
            self.viewport_pos = Axis::Vertical
                .pack(
                    progress * Axis::Vertical.major(content_size - portal_size),
                    Axis::Vertical.minor_pos(self.viewport_pos),
                )
                .into();
            ctx.request_layout();
        }
    }

    // TODO - handle Home/End keys, etc
    fn on_text_event(&mut self, ctx: &mut EventCtx, event: &TextEvent) {
        self.child.on_text_event(ctx, event);
        self.scrollbar_horizontal.on_text_event(ctx, event);
        self.scrollbar_vertical.on_text_event(ctx, event);
    }

    fn on_status_change(&mut self, _ctx: &mut LifeCycleCtx, _event: &StatusChange) {}

    fn lifecycle(&mut self, ctx: &mut LifeCycleCtx, event: &LifeCycle) {
        match event {
            LifeCycle::WidgetAdded => {
                ctx.register_as_portal();
            }
            //TODO
            //LifeCycle::RequestPanToChild(target_rect) => {}
            _ => {}
        }

        self.child.lifecycle(ctx, event);
        self.scrollbar_horizontal.lifecycle(ctx, event);
        self.scrollbar_vertical.lifecycle(ctx, event);
    }

    fn layout(&mut self, ctx: &mut LayoutCtx, bc: &BoxConstraints) -> Size {
        let min_child_size = if self.must_fill { bc.min() } else { Size::ZERO };
        let mut max_child_size = bc.max();
        if !self.constrain_horizontal {
            max_child_size.width = f64::INFINITY
        };
        if !self.constrain_vertical {
            max_child_size.height = f64::INFINITY
        };

        let child_bc = BoxConstraints::new(min_child_size, max_child_size);

        let content_size = self.child.layout(ctx, &child_bc);
        let portal_size = bc.constrain(content_size);

        // TODO - document better
        // Recompute the portal offset for the new layout
        self.set_viewport_pos_raw(portal_size, content_size, self.viewport_pos);
        // TODO - recompute portal progress

        ctx.place_child(&mut self.child, Point::new(0.0, -self.viewport_pos.y));

        self.scrollbar_horizontal_visible =
            !self.constrain_horizontal && portal_size.width < content_size.width;
        self.scrollbar_vertical_visible =
            !self.constrain_vertical && portal_size.height < content_size.height;

        if self.scrollbar_horizontal_visible {
            self.scrollbar_horizontal.widget_mut().portal_size = portal_size.width;
            self.scrollbar_horizontal.widget_mut().content_size = content_size.width;
            let scrollbar_size = self.scrollbar_horizontal.layout(ctx, bc);
            ctx.place_child(
                &mut self.scrollbar_horizontal,
                Point::new(0.0, portal_size.height - scrollbar_size.height),
            );
        } else {
            ctx.skip_child(&mut self.scrollbar_horizontal);
        }
        if self.scrollbar_vertical_visible {
            self.scrollbar_vertical.widget_mut().portal_size = portal_size.height;
            self.scrollbar_vertical.widget_mut().content_size = content_size.height;
            let scrollbar_size = self.scrollbar_vertical.layout(ctx, bc);
            ctx.place_child(
                &mut self.scrollbar_vertical,
                Point::new(portal_size.width - scrollbar_size.width, 0.0),
            );
        } else {
            ctx.skip_child(&mut self.scrollbar_vertical);
        }

        portal_size
    }

    fn paint(&mut self, ctx: &mut PaintCtx, scene: &mut Scene) {
        // TODO - also clip the invalidated region
        let clip_rect = ctx.size().to_rect();

        scene.push_layer(BlendMode::default(), 1., Affine::IDENTITY, &clip_rect);
        self.child.paint(ctx, scene);
        scene.pop_layer();

        if self.scrollbar_horizontal_visible {
            self.scrollbar_horizontal.paint(ctx, scene);
        } else {
            ctx.skip_child(&mut self.scrollbar_horizontal);
        }
        if self.scrollbar_vertical_visible {
            self.scrollbar_vertical.paint(ctx, scene);
        } else {
            ctx.skip_child(&mut self.scrollbar_vertical);
        }
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
    use insta::assert_debug_snapshot;

    use super::*;
    use crate::assert_render_snapshot;
    use crate::testing::{widget_ids, TestHarness};
    use crate::widget::{Button, Flex, SizedBox};

    fn button(text: &str) -> impl Widget {
        SizedBox::new(Button::new(text)).width(70.0).height(40.0)
    }

    // TODO - This test takes too long right now
    #[test]
    #[ignore]
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

        let mut harness = TestHarness::create_with_size(widget, Size::new(400., 400.));

        assert_debug_snapshot!(harness.root_widget());
        assert_render_snapshot!(harness, "button_list_no_scroll");

        harness.edit_root_widget(|mut portal| {
            let mut portal = portal.downcast::<Portal<Flex>>().unwrap();
            portal.set_viewport_pos(Point::new(0.0, 130.0))
        });

        assert_render_snapshot!(harness, "button_list_scrolled");

        let item_3_rect = harness.get_widget(item_3_id).state().layout_rect();
        harness.edit_root_widget(|mut portal| {
            let mut portal = portal.downcast::<Portal<Flex>>().unwrap();
            portal.pan_viewport_to(item_3_rect);
        });

        assert_render_snapshot!(harness, "button_list_scroll_to_item_3");

        let item_13_rect = harness.get_widget(item_13_id).state().layout_rect();
        harness.edit_root_widget(|mut portal| {
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
