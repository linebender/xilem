// Copyright 2022 the Xilem Authors
// SPDX-License-Identifier: Apache-2.0

use accesskit::{Node, Role};
use tracing::{Span, trace_span};
use ui_events::pointer::PointerButtonEvent;
use vello::Scene;
use vello::kurbo::{Point, Rect, Size};

use crate::core::{
    AccessCtx, AccessEvent, AllowRawMut, Axis, BoxConstraints, ChildrenIds, EventCtx, LayoutCtx,
    NoAction, PaintCtx, PointerEvent, PropertiesMut, PropertiesRef, RegisterCtx, TextEvent, Update,
    UpdateCtx, Widget, WidgetId, WidgetMut,
};
use crate::theme;
use crate::util::{fill_color, include_screenshot, stroke};

// TODO
// - Fade scrollbars? Find out how Linux/macOS/Windows do it
// - Rename cursor to oval/rect/bar/grabber/grabbybar
// - Rename progress to something more descriptive
// - Document names
// - Document invariants

/// A scrollbar.
///
#[doc = include_screenshot!("scrollbar_default.png", "Vertical scrollbar.")]
pub struct ScrollBar {
    axis: Axis,
    pub(crate) cursor_progress: f64,
    pub(crate) moved: bool,
    pub(crate) portal_size: f64,
    pub(crate) content_size: f64,
    grab_anchor: Option<f64>,
}

// --- MARK: BUILDERS
impl ScrollBar {
    #[expect(missing_docs, reason = "TODO")]
    pub fn new(axis: Axis, portal_size: f64, content_size: f64) -> Self {
        Self {
            axis,
            cursor_progress: 0.0,
            moved: false,
            portal_size,
            content_size,
            grab_anchor: None,
        }
    }

    /// Returns how far the scrollbar is from its initial point.
    ///
    /// Values range from 0.0 (beginning) to 1.0 (end).
    pub fn cursor_progress(&self) -> f64 {
        self.cursor_progress
    }
}

impl ScrollBar {
    fn get_cursor_rect(&self, layout_size: Size, min_length: f64) -> Rect {
        // TODO - handle invalid sizes
        let size_ratio = self.portal_size / self.content_size;
        let size_ratio = size_ratio.clamp(0.0, 1.0);

        let cursor_length = size_ratio * self.axis.major(layout_size);
        let cursor_length = cursor_length.max(min_length);

        let empty_space_length = (1.0 - size_ratio) * self.axis.major(layout_size);
        let cursor_pos_major = self.cursor_progress * empty_space_length;

        let cursor_pos = self.axis.pack(cursor_pos_major, 0.0);
        let cursor_size = self.axis.pack(cursor_length, self.axis.minor(layout_size));

        Rect::from_origin_size(cursor_pos, cursor_size)
    }

    fn progress_from_mouse_pos(
        &self,
        layout_size: Size,
        min_length: f64,
        anchor: f64,
        mouse_pos: Point,
    ) -> f64 {
        // TODO - handle invalid sizes
        let size_ratio = self.portal_size / self.content_size;
        let size_ratio = size_ratio.clamp(0.0, 1.0);

        let cursor_rect = self.get_cursor_rect(layout_size, min_length);

        // invariant: cursor_x == progress * (1 - size_ratio) * layout_width
        // invariant: cursor_x + anchor * cursor_width == mouse_x

        let cursor_width = self.axis.major(cursor_rect.size());
        let new_cursor_pos_major = self.axis.major_pos(mouse_pos) - anchor * cursor_width;

        let empty_space_length = (1.0 - size_ratio) * self.axis.major(layout_size);
        let new_cursor_progress = new_cursor_pos_major / empty_space_length;

        new_cursor_progress.clamp(0.0, 1.0)
    }
}

// --- MARK: WIDGETMUT
impl ScrollBar {
    // TODO - Remove?
    #[expect(missing_docs, reason = "TODO")]
    pub fn set_sizes(this: &mut WidgetMut<'_, Self>, portal_size: f64, content_size: f64) {
        this.widget.portal_size = portal_size;
        this.widget.content_size = content_size;
        this.ctx.request_render();
    }

    // TODO - Remove?
    #[expect(missing_docs, reason = "TODO")]
    pub fn set_content_size(this: &mut WidgetMut<'_, Self>, content_size: f64) {
        // TODO - cursor_progress
        this.widget.content_size = content_size;
        this.ctx.request_render();
    }
}

// --- MARK: IMPL WIDGET
impl Widget for ScrollBar {
    type Action = NoAction;

    fn on_pointer_event(
        &mut self,
        ctx: &mut EventCtx<'_>,
        _props: &mut PropertiesMut<'_>,
        event: &PointerEvent,
    ) {
        match event {
            PointerEvent::Down(PointerButtonEvent { state, .. }) => {
                ctx.capture_pointer();

                let cursor_min_length = theme::SCROLLBAR_MIN_SIZE;
                let cursor_rect = self.get_cursor_rect(ctx.size(), cursor_min_length);
                let mouse_pos = ctx.local_position(state.position);
                if cursor_rect.contains(mouse_pos) {
                    let (z0, z1) = self.axis.major_span(cursor_rect);
                    let mouse_major = self.axis.major_pos(mouse_pos);
                    self.grab_anchor = Some((mouse_major - z0) / (z1 - z0));
                } else {
                    self.cursor_progress =
                        self.progress_from_mouse_pos(ctx.size(), cursor_min_length, 0.5, mouse_pos);
                    self.moved = true;
                    self.grab_anchor = Some(0.5);
                };
                ctx.request_render();
            }
            PointerEvent::Move(u) => {
                if let Some(grab_anchor) = self.grab_anchor {
                    let cursor_min_length = theme::SCROLLBAR_MIN_SIZE;
                    self.cursor_progress = self.progress_from_mouse_pos(
                        ctx.size(),
                        cursor_min_length,
                        grab_anchor,
                        ctx.local_position(u.current.position),
                    );
                    self.moved = true;
                }
                ctx.request_render();
            }
            PointerEvent::Up { .. } | PointerEvent::Cancel(..) => {
                self.grab_anchor = None;
                ctx.request_render();
            }
            _ => {}
        }
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
        // TODO - Handle scroll-related events?
    }

    fn register_children(&mut self, _ctx: &mut RegisterCtx<'_>) {}

    fn update(
        &mut self,
        _ctx: &mut UpdateCtx<'_>,
        _props: &mut PropertiesMut<'_>,
        _event: &Update,
    ) {
    }

    fn layout(
        &mut self,
        _ctx: &mut LayoutCtx<'_>,
        _props: &mut PropertiesMut<'_>,
        bc: &BoxConstraints,
    ) -> Size {
        // TODO - handle resize

        let scrollbar_width = theme::SCROLLBAR_WIDTH;
        let cursor_padding = theme::SCROLLBAR_PAD;
        self.axis
            .pack(
                self.axis.major(bc.max()),
                scrollbar_width + cursor_padding * 2.0,
            )
            .into()
    }

    fn paint(&mut self, ctx: &mut PaintCtx<'_>, _props: &PropertiesRef<'_>, scene: &mut Scene) {
        let radius = theme::SCROLLBAR_RADIUS;
        let edge_width = theme::SCROLLBAR_EDGE_WIDTH;
        let cursor_padding = theme::SCROLLBAR_PAD;
        let cursor_min_length = theme::SCROLLBAR_MIN_SIZE;

        let (inset_x, inset_y) = self.axis.pack(0.0, cursor_padding);
        let cursor_rect = self
            .get_cursor_rect(ctx.size(), cursor_min_length)
            .inset((-inset_x, -inset_y))
            .to_rounded_rect(radius);

        fill_color(scene, &cursor_rect, theme::SCROLLBAR_COLOR);
        stroke(
            scene,
            &cursor_rect,
            theme::SCROLLBAR_BORDER_COLOR,
            edge_width,
        );
    }

    fn accessibility_role(&self) -> Role {
        Role::ScrollBar
    }

    fn accessibility(
        &mut self,
        _ctx: &mut AccessCtx<'_>,
        _props: &PropertiesRef<'_>,
        _node: &mut Node,
    ) {
        // TODO
        // Use set_scroll_x/y_min/max?
    }

    fn children_ids(&self) -> ChildrenIds {
        ChildrenIds::new()
    }

    fn make_trace_span(&self, id: WidgetId) -> Span {
        trace_span!("ScrollBar", id = id.trace())
    }
}

impl AllowRawMut for ScrollBar {}

// --- MARK: TESTS
#[cfg(test)]
mod tests {
    use masonry_core::core::NewWidget;

    use super::*;
    use crate::core::PointerButton;
    use crate::testing::{TestHarness, assert_render_snapshot};
    use crate::theme::default_property_set;

    #[test]
    fn simple_scrollbar() {
        let widget = NewWidget::new(ScrollBar::new(Axis::Vertical, 200.0, 600.0));

        let mut harness =
            TestHarness::create_with_size(default_property_set(), widget, Size::new(50.0, 200.0));
        let scrollbar_id = harness.root_id();

        assert_render_snapshot!(harness, "scrollbar_default");

        assert!(harness.pop_action_erased().is_none());

        harness.mouse_click_on(scrollbar_id);
        // TODO - Scroll action?
        assert!(harness.pop_action_erased().is_none());

        assert_render_snapshot!(harness, "scrollbar_middle");

        harness.mouse_button_press(PointerButton::Primary);
        harness.mouse_move(Point::new(30.0, 150.0));

        assert_render_snapshot!(harness, "scrollbar_down");

        harness.mouse_move(Point::new(30.0, 300.0));
        assert_render_snapshot!(harness, "scrollbar_bottom");
    }

    #[test]
    fn horizontal_scrollbar() {
        let widget = NewWidget::new(ScrollBar::new(Axis::Horizontal, 200.0, 600.0));

        let mut harness =
            TestHarness::create_with_size(default_property_set(), widget, Size::new(200.0, 50.0));
        let scrollbar_id = harness.root_id();

        assert_render_snapshot!(harness, "scrollbar_horizontal");

        assert!(harness.pop_action_erased().is_none());

        harness.mouse_click_on(scrollbar_id);
        // TODO - Scroll action?
        assert!(harness.pop_action_erased().is_none());

        assert_render_snapshot!(harness, "scrollbar_horizontal_middle");
    }

    // TODO - Add "portal larger than content" test

    // TODO - Add WidgetMut tests
}
