// Copyright 2020 the Xilem Authors and the Druid Authors
// SPDX-License-Identifier: Apache-2.0

use std::ops::Range;

use accesskit::{Node, Role};
use dpi::PhysicalPosition;
use tracing::{Span, trace_span};
use ui_events::pointer::PointerScrollEvent;
use vello::Scene;
use vello::kurbo::{Point, Rect, Size, Vec2};

use crate::core::{
    AccessCtx, AccessEvent, Axis, BoxConstraints, ChildrenIds, ComposeCtx, EventCtx, FromDynWidget,
    LayoutCtx, NewWidget, NoAction, PaintCtx, PointerEvent, PropertiesMut, PropertiesRef,
    RegisterCtx, ScrollDelta, TextEvent, Update, UpdateCtx, Widget, WidgetId, WidgetMut, WidgetPod,
};
use crate::widgets::ScrollBar;

// TODO - refactor - see https://github.com/linebender/xilem/issues/366
// TODO - rename "Portal" to "ScrollPortal"?
// TODO - Document which cases need request_layout, request_compose and request_render
// Conceptually, a Portal is a widget giving a restricted view of a child widget
// Imagine a very large widget, and a rect that represents the part of the widget we see
#[expect(missing_docs, reason = "TODO")]
pub struct Portal<W: Widget + ?Sized> {
    child: WidgetPod<W>,
    content_size: Size,
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

// --- MARK: BUILDERS
impl<W: Widget + ?Sized> Portal<W> {
    #[expect(missing_docs, reason = "TODO")]
    pub fn new(child: NewWidget<W>) -> Self {
        Self {
            child: child.to_pod(),
            content_size: Size::ZERO,
            viewport_pos: Point::ORIGIN,
            constrain_horizontal: false,
            constrain_vertical: false,
            must_fill: false,
            // TODO - remove (TODO: why?)
            scrollbar_horizontal: WidgetPod::new(ScrollBar::new(Axis::Horizontal, 1.0, 1.0)),
            scrollbar_horizontal_visible: false,
            scrollbar_vertical: WidgetPod::new(ScrollBar::new(Axis::Vertical, 1.0, 1.0)),
            scrollbar_vertical_visible: false,
        }
    }

    #[expect(missing_docs, reason = "TODO")]
    pub fn get_viewport_pos(&self) -> Point {
        self.viewport_pos
    }

    // TODO - rewrite doc
    /// Builder-style method for deciding whether to constrain the child vertically.
    ///
    /// The default is `false`.
    ///
    /// This setting affects how a `Portal` lays out its child.
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
    /// The default is `false`.
    pub fn constrain_horizontal(mut self, constrain: bool) -> Self {
        self.constrain_horizontal = constrain;
        self
    }

    /// Builder-style method to set whether the child must fill the view.
    ///
    /// If `false` (the default) there is no minimum constraint on the child's
    /// size. If `true`, the child is passed the same minimum constraints as
    /// the `Portal`.
    pub fn content_must_fill(mut self, must_fill: bool) -> Self {
        self.must_fill = must_fill;
        self
    }
}

pub(crate) fn compute_pan_range(mut viewport: Range<f64>, target: Range<f64>) -> Range<f64> {
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

impl<W: Widget + ?Sized> Portal<W> {
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

    // Note - Rect is in child coordinates
    // TODO - Merge with pan_viewport_to
    // Right now these functions are just different enough to be a pain to merge.
    fn pan_viewport_to_raw(&mut self, portal_size: Size, content_size: Size, target: Rect) -> bool {
        let viewport = Rect::from_origin_size(self.viewport_pos, portal_size);

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

        self.set_viewport_pos_raw(portal_size, content_size, Point::new(new_pos_x, new_pos_y))
    }
}

// --- MARK: WIDGETMUT
impl<W: Widget + FromDynWidget + ?Sized> Portal<W> {
    #[expect(missing_docs, reason = "TODO")]
    pub fn child_mut<'t>(this: &'t mut WidgetMut<'_, Self>) -> WidgetMut<'t, W> {
        this.ctx.get_mut(&mut this.widget.child)
    }

    #[expect(missing_docs, reason = "TODO")]
    pub fn horizontal_scrollbar_mut<'t>(
        this: &'t mut WidgetMut<'_, Self>,
    ) -> WidgetMut<'t, ScrollBar> {
        this.ctx.get_mut(&mut this.widget.scrollbar_horizontal)
    }

    #[expect(missing_docs, reason = "TODO")]
    pub fn vertical_scrollbar_mut<'t>(
        this: &'t mut WidgetMut<'_, Self>,
    ) -> WidgetMut<'t, ScrollBar> {
        this.ctx.get_mut(&mut this.widget.scrollbar_vertical)
    }

    // TODO - rewrite doc
    /// Set whether to constrain the child horizontally.
    pub fn set_constrain_horizontal(this: &mut WidgetMut<'_, Self>, constrain: bool) {
        this.widget.constrain_horizontal = constrain;
        this.ctx.request_layout();
    }

    /// Set whether to constrain the child vertically.
    pub fn set_constrain_vertical(this: &mut WidgetMut<'_, Self>, constrain: bool) {
        this.widget.constrain_vertical = constrain;
        this.ctx.request_layout();
    }

    /// Set whether the child's size must be greater than or equal the size of
    /// the `Portal`.
    ///
    /// See [`content_must_fill`] for more details.
    ///
    /// [`content_must_fill`]: Portal::content_must_fill
    pub fn set_content_must_fill(this: &mut WidgetMut<'_, Self>, must_fill: bool) {
        this.widget.must_fill = must_fill;
        this.ctx.request_layout();
    }

    #[expect(missing_docs, reason = "TODO")]
    pub fn set_viewport_pos(this: &mut WidgetMut<'_, Self>, position: Point) -> bool {
        let portal_size = this.ctx.size();
        let content_size = this.ctx.get_mut(&mut this.widget.child).ctx.size();

        let pos_changed = this
            .widget
            .set_viewport_pos_raw(portal_size, content_size, position);
        if pos_changed {
            let progress_x = this.widget.viewport_pos.x / (content_size - portal_size).width;
            Self::horizontal_scrollbar_mut(this).widget.cursor_progress = progress_x;
            Self::horizontal_scrollbar_mut(this).ctx.request_render();
            let progress_y = this.widget.viewport_pos.y / (content_size - portal_size).height;
            Self::vertical_scrollbar_mut(this).widget.cursor_progress = progress_y;
            Self::vertical_scrollbar_mut(this).ctx.request_render();
            this.ctx.request_layout();
        }
        pos_changed
    }

    #[expect(missing_docs, reason = "TODO")]
    pub fn pan_viewport_by(this: &mut WidgetMut<'_, Self>, translation: Vec2) -> bool {
        Self::set_viewport_pos(this, this.widget.viewport_pos + translation)
    }

    #[expect(missing_docs, reason = "TODO")]
    // Note - Rect is in child coordinates
    pub fn pan_viewport_to(this: &mut WidgetMut<'_, Self>, target: Rect) -> bool {
        let viewport = Rect::from_origin_size(this.widget.viewport_pos, this.ctx.size());

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

        Self::set_viewport_pos(this, Point::new(new_pos_x, new_pos_y))
    }
}

// --- MARK: IMPL WIDGET
impl<W: Widget + FromDynWidget + ?Sized> Widget for Portal<W> {
    type Action = NoAction;

    fn on_pointer_event(
        &mut self,
        ctx: &mut EventCtx<'_>,
        _props: &mut PropertiesMut<'_>,
        event: &PointerEvent,
    ) {
        let portal_size = ctx.size();
        let content_size = self.content_size;

        match *event {
            PointerEvent::Scroll(PointerScrollEvent { delta, .. }) => {
                // TODO - Remove reference to scale factor.
                // See https://github.com/linebender/xilem/issues/1264
                let delta = match delta {
                    ScrollDelta::PixelDelta(PhysicalPosition::<f64> { x, y }) => -Vec2 { x, y },
                    ScrollDelta::LineDelta(x, y) => {
                        -Vec2 {
                            x: x as f64,
                            y: y as f64,
                        } * 120.0
                    }
                    _ => Vec2::ZERO,
                } * ctx.get_scale_factor();
                self.set_viewport_pos_raw(portal_size, content_size, self.viewport_pos + delta);
                ctx.request_compose();

                // TODO - horizontal scrolling?
                let (scrollbar, mut scrollbar_ctx) = ctx.get_raw_mut(&mut self.scrollbar_vertical);
                scrollbar.cursor_progress =
                    self.viewport_pos.y / (content_size - portal_size).height;
                scrollbar_ctx.request_render();
            }
            _ => (),
        }

        // This section works because events are propagated up. So if the scrollbar got
        // pointer events, then its event method has already been called by the time this runs.
        let mut scrollbar_moved = false;
        {
            let (scrollbar, _) = ctx.get_raw_mut(&mut self.scrollbar_horizontal);
            if scrollbar.moved {
                scrollbar.moved = false;

                let progress = scrollbar.cursor_progress;
                self.viewport_pos = Axis::Horizontal
                    .pack(
                        progress * Axis::Horizontal.major(content_size - portal_size),
                        Axis::Horizontal.minor_pos(self.viewport_pos),
                    )
                    .into();
                scrollbar_moved = true;
            }
        }
        {
            let (scrollbar, _) = ctx.get_raw_mut(&mut self.scrollbar_vertical);
            if scrollbar.moved {
                scrollbar.moved = false;

                let progress = scrollbar.cursor_progress;
                self.viewport_pos = Axis::Vertical
                    .pack(
                        progress * Axis::Vertical.major(content_size - portal_size),
                        Axis::Vertical.minor_pos(self.viewport_pos),
                    )
                    .into();
                scrollbar_moved = true;
            }
        }

        if scrollbar_moved {
            ctx.request_compose();
        }
    }

    // TODO - handle Home/End keys, etc
    fn on_text_event(
        &mut self,
        _ctx: &mut EventCtx<'_>,
        _props: &mut PropertiesMut<'_>,
        _event: &TextEvent,
    ) {
    }

    // TODO - Handle scroll-related events?
    fn on_access_event(
        &mut self,
        _ctx: &mut EventCtx<'_>,
        _props: &mut PropertiesMut<'_>,
        _event: &AccessEvent,
    ) {
    }

    fn register_children(&mut self, ctx: &mut RegisterCtx<'_>) {
        ctx.register_child(&mut self.child);
        ctx.register_child(&mut self.scrollbar_horizontal);
        ctx.register_child(&mut self.scrollbar_vertical);
    }

    fn update(&mut self, ctx: &mut UpdateCtx<'_>, _props: &mut PropertiesMut<'_>, event: &Update) {
        match event {
            Update::RequestPanToChild(target) => {
                let portal_size = ctx.size();
                let content_size = self.content_size;

                self.pan_viewport_to_raw(portal_size, content_size, *target);
                ctx.request_compose();

                // TODO - There's a lot of code here that's duplicated from the `MouseWheel`
                // event in `on_pointer_event`.
                // Because this code directly manipulates child widgets, it's hard to factor
                // it out.
                let (scrollbar, mut scrollbar_ctx) = ctx.get_raw_mut(&mut self.scrollbar_vertical);
                scrollbar.cursor_progress =
                    self.viewport_pos.y / (content_size - portal_size).height;
                scrollbar_ctx.request_render();

                std::mem::drop(scrollbar_ctx);

                let (scrollbar, mut scrollbar_ctx) =
                    ctx.get_raw_mut(&mut self.scrollbar_horizontal);
                scrollbar.cursor_progress =
                    self.viewport_pos.x / (content_size - portal_size).width;
                scrollbar_ctx.request_render();
            }
            _ => {}
        }
    }

    fn layout(
        &mut self,
        ctx: &mut LayoutCtx<'_>,
        _props: &mut PropertiesMut<'_>,
        bc: &BoxConstraints,
    ) -> Size {
        // TODO - How Portal handles BoxConstraints is due for a rework
        let min_child_size = if self.must_fill { bc.min() } else { Size::ZERO };
        let max_child_size = bc.max();

        let child_bc = BoxConstraints::new(min_child_size, max_child_size);

        let content_size = ctx.run_layout(&mut self.child, &child_bc);
        let portal_size = bc.constrain(content_size);

        self.content_size = content_size;

        // TODO - document better
        // Recompute the portal offset for the new layout
        self.set_viewport_pos_raw(portal_size, content_size, self.viewport_pos);
        // TODO - recompute portal progress

        ctx.set_clip_path(portal_size.to_rect());

        ctx.place_child(&mut self.child, Point::ZERO);

        self.scrollbar_horizontal_visible =
            !self.constrain_horizontal && portal_size.width < content_size.width;
        self.scrollbar_vertical_visible =
            !self.constrain_vertical && portal_size.height < content_size.height;

        ctx.set_stashed(
            &mut self.scrollbar_horizontal,
            !self.scrollbar_horizontal_visible,
        );
        if self.scrollbar_horizontal_visible {
            let (scrollbar, _) = ctx.get_raw_mut(&mut self.scrollbar_horizontal);
            scrollbar.portal_size = portal_size.width;
            scrollbar.content_size = content_size.width;
            // TODO - request paint for scrollbar?

            let scrollbar_size = ctx.run_layout(&mut self.scrollbar_horizontal, bc);
            ctx.place_child(
                &mut self.scrollbar_horizontal,
                Point::new(0.0, portal_size.height - scrollbar_size.height),
            );
        }

        ctx.set_stashed(
            &mut self.scrollbar_vertical,
            !self.scrollbar_vertical_visible,
        );
        if self.scrollbar_vertical_visible {
            let (scrollbar, _) = ctx.get_raw_mut(&mut self.scrollbar_vertical);
            scrollbar.portal_size = portal_size.height;
            scrollbar.content_size = content_size.height;
            // TODO - request paint for scrollbar?

            let scrollbar_size = ctx.run_layout(&mut self.scrollbar_vertical, bc);
            ctx.place_child(
                &mut self.scrollbar_vertical,
                Point::new(portal_size.width - scrollbar_size.width, 0.0),
            );
        }

        portal_size
    }

    fn compose(&mut self, ctx: &mut ComposeCtx<'_>) {
        ctx.set_child_scroll_translation(&mut self.child, Vec2::new(0.0, -self.viewport_pos.y));
    }

    fn paint(&mut self, _ctx: &mut PaintCtx<'_>, _props: &PropertiesRef<'_>, _scene: &mut Scene) {}

    fn accessibility_role(&self) -> Role {
        Role::GenericContainer
    }

    fn accessibility(
        &mut self,
        _ctx: &mut AccessCtx<'_>,
        _props: &PropertiesRef<'_>,
        node: &mut Node,
    ) {
        node.set_clips_children();
    }

    fn children_ids(&self) -> ChildrenIds {
        ChildrenIds::from_slice(&[
            self.child.id(),
            self.scrollbar_vertical.id(),
            self.scrollbar_horizontal.id(),
        ])
    }

    fn make_trace_span(&self, id: WidgetId) -> Span {
        trace_span!("Portal", id = id.trace())
    }
}

// --- MARK: TESTS
#[cfg(test)]
mod tests {
    use masonry_core::core::WidgetTag;

    use super::*;
    use crate::properties::types::AsUnit;
    use crate::testing::{TestHarness, assert_render_snapshot};
    use crate::theme::default_property_set;
    use crate::widgets::{Button, Flex, SizedBox};

    fn button(text: &'static str) -> impl Widget {
        SizedBox::new(Button::with_text(text).with_auto_id())
            .width(70.px())
            .height(40.px())
    }

    #[test]
    fn button_list() {
        let button_3 = WidgetTag::new("button-3");
        let button_13 = WidgetTag::new("button-13");

        let widget = Portal::new(NewWidget::new(
            Flex::column()
                .with_child(button("Item 1").with_auto_id())
                .with_spacer(10.px())
                .with_child(button("Item 2").with_auto_id())
                .with_spacer(10.px())
                .with_child(NewWidget::new_with_tag(button("Item 3"), button_3))
                .with_spacer(10.px())
                .with_child(button("Item 4").with_auto_id())
                .with_spacer(10.px())
                .with_child(button("Item 5").with_auto_id())
                .with_spacer(10.px())
                .with_child(button("Item 6").with_auto_id())
                .with_spacer(10.px())
                .with_child(button("Item 7").with_auto_id())
                .with_spacer(10.px())
                .with_child(button("Item 8").with_auto_id())
                .with_spacer(10.px())
                .with_child(button("Item 9").with_auto_id())
                .with_spacer(10.px())
                .with_child(button("Item 10").with_auto_id())
                .with_spacer(10.px())
                .with_child(button("Item 11").with_auto_id())
                .with_spacer(10.px())
                .with_child(button("Item 12").with_auto_id())
                .with_spacer(10.px())
                .with_child(NewWidget::new_with_tag(button("Item 13"), button_13))
                .with_spacer(10.px())
                .with_child(button("Item 14").with_auto_id())
                .with_spacer(10.px()),
        ))
        .with_auto_id();

        let mut harness =
            TestHarness::create_with_size(default_property_set(), widget, Size::new(400., 400.));

        assert_render_snapshot!(harness, "portal_button_list_no_scroll");

        harness.edit_root_widget(|mut portal| {
            Portal::set_viewport_pos(&mut portal, Point::new(0.0, 130.0))
        });

        assert_render_snapshot!(harness, "portal_button_list_scrolled");

        let item_3_rect = harness.get_widget(button_3).ctx().local_layout_rect();
        harness.edit_root_widget(|mut portal| {
            Portal::pan_viewport_to(&mut portal, item_3_rect);
        });

        assert_render_snapshot!(harness, "portal_button_list_scroll_to_item_3");

        let item_13_rect = harness.get_widget(button_13).ctx().local_layout_rect();
        harness.edit_root_widget(|mut portal| {
            Portal::pan_viewport_to(&mut portal, item_13_rect);
        });

        assert_render_snapshot!(harness, "portal_button_list_scroll_to_item_13");
    }

    #[test]
    fn scroll_into_view() {
        let button_tag = WidgetTag::new("hidden-button");

        let widget = Portal::new(
            Flex::column()
                .with_spacer(500.px())
                .with_child(NewWidget::new_with_tag(
                    Button::with_text("Fully visible"),
                    button_tag,
                ))
                .with_spacer(500.px())
                .with_auto_id(),
        )
        .with_auto_id();

        let mut harness =
            TestHarness::create_with_size(default_property_set(), widget, Size::new(200., 200.));
        let button_id = harness.get_widget(button_tag).id();

        harness.scroll_into_view(button_id);
        assert_render_snapshot!(harness, "portal_scrolled_button_into_view");
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
