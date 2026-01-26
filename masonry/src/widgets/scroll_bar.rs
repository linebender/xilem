// Copyright 2022 the Xilem Authors
// SPDX-License-Identifier: Apache-2.0

use accesskit::{Node, Role};
use include_doc_path::include_doc_path;
use tracing::{Span, trace_span};
use vello::Scene;

use crate::core::keyboard::{Key, KeyState, NamedKey};
use crate::core::{
    AccessCtx, AccessEvent, AllowRawMut, ChildrenIds, EventCtx, LayoutCtx, MeasureCtx, NoAction,
    PaintCtx, PointerButtonEvent, PointerEvent, PointerUpdate, PropertiesMut, PropertiesRef,
    RegisterCtx, TextEvent, Update, UpdateCtx, Widget, WidgetId, WidgetMut,
};
use crate::kurbo::{Axis, Point, Rect, Size};
use crate::layout::LenReq;
use crate::theme;
use crate::util::{fill_color, stroke};

// TODO
// - Fade scrollbars? Find out how Linux/macOS/Windows do it
// - Rename cursor to oval/rect/bar/grabber/grabbybar
// - Rename progress to something more descriptive
// - Document names
// - Document invariants

/// A scrollbar.
///
#[doc = concat!(
    "![Vertical scrollbar](",
    include_doc_path!("screenshots/scrollbar_default.png"),
    ")",
)]
/// This widget does not directly scroll any content. Instead, it exposes its position via
/// [`cursor_progress`](Self::cursor_progress) and sets `moved` to `true` whenever user input changes
/// that position. A parent scroll container (such as [`Portal`](crate::widgets::Portal)) is expected
/// to observe `moved` and update its viewport accordingly.
///
/// ## Keyboard and accessibility
///
/// Scrollbars are focusable and support basic keyboard navigation (arrow keys, PageUp/Down,
/// Home/End depending on axis), and expose scroll state in the accessibility tree.
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
    /// Creates a new scrollbar.
    ///
    /// - `portal_size`: Size of the scrolling container in the relevant axis.
    /// - `content_size`: Size of the child in the relevant axis. Usually exceeds `portal_size` in cases where the scrollbar is visible.
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
}

// --- MARK: METHODS
impl ScrollBar {
    /// Returns how far the scrollbar is from its initial point.
    ///
    /// Values range from 0.0 (beginning) to 1.0 (end).
    pub fn cursor_progress(&self) -> f64 {
        self.cursor_progress
    }

    /// Returns `(cursor_length, empty_space_length)`.
    ///
    /// `cursor_Length` is guaranteed to be at least `min_length`
    /// and the remainder of the layout length is `empty_space_length`.
    fn lengths(&self, layout_size: Size, min_length: f64) -> (f64, f64) {
        let size_ratio = if self.content_size != 0. {
            self.portal_size / self.content_size
        } else {
            1.
        };
        let size_ratio = size_ratio.clamp(0.0, 1.0);

        let cursor_length = (size_ratio * layout_size.get_coord(self.axis)).max(min_length);
        let empty_space_length = layout_size.get_coord(self.axis) - cursor_length;

        (cursor_length, empty_space_length)
    }

    fn cursor_rect(&self, layout_size: Size, min_length: f64) -> Rect {
        // TODO - handle invalid sizes
        let (cursor_length, empty_space_length) = self.lengths(layout_size, min_length);

        let cursor_pos_major = self.cursor_progress * empty_space_length;
        let cursor_pos = self.axis.pack_point(cursor_pos_major, 0.0);
        let cursor_size_minor = layout_size.get_coord(self.axis.cross());
        let cursor_size = self.axis.pack_size(cursor_length, cursor_size_minor);

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
        let (cursor_length, empty_space_length) = self.lengths(layout_size, min_length);

        let new_cursor_pos_major = mouse_pos.get_coord(self.axis) - anchor * cursor_length;
        let new_cursor_progress = new_cursor_pos_major / empty_space_length;

        new_cursor_progress.clamp(0.0, 1.0)
    }

    fn scroll_range(&self) -> f64 {
        (self.content_size - self.portal_size).max(0.0)
    }

    fn set_cursor_progress(&mut self, new_progress: f64) -> bool {
        let new_progress = new_progress.clamp(0.0, 1.0);
        if (new_progress - self.cursor_progress).abs() > 1e-12 {
            self.cursor_progress = new_progress;
            self.moved = true;
            true
        } else {
            false
        }
    }

    fn adjust_by_pixels(&mut self, delta_pixels: f64) -> bool {
        let scroll_range = self.scroll_range();
        if scroll_range <= 1e-12 {
            return false;
        }
        // Translate from a pixel delta in content space to a normalized cursor progress delta.
        self.set_cursor_progress(self.cursor_progress + delta_pixels / scroll_range)
    }
}

// --- MARK: WIDGETMUT
impl ScrollBar {
    /// Updates the sizes of the widgets being represented by the scrollbar.
    /// - `portal_size`: Size of the scrolling container in the relevant axis.
    /// - `content_size`: Size of the child in the relevant axis. Usually exceeds `portal_size` in cases where the scrollbar is visible.
    // TODO - Remove?
    pub fn set_sizes(this: &mut WidgetMut<'_, Self>, portal_size: f64, content_size: f64) {
        this.widget.portal_size = portal_size;
        this.widget.content_size = content_size;
        this.ctx.request_render();
    }

    /// Updates the size of the child widget being represented by the scrollbar.
    // TODO - Remove?
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
                let cursor_rect = self.cursor_rect(ctx.size(), cursor_min_length);
                let mouse_pos = ctx.local_position(state.position);
                let mut changed = false;
                if cursor_rect.contains(mouse_pos) {
                    let (c0, c1) = cursor_rect.get_coords(self.axis);
                    let mouse_major = mouse_pos.get_coord(self.axis);
                    self.grab_anchor = Some((mouse_major - c0) / (c1 - c0));
                } else {
                    let progress =
                        self.progress_from_mouse_pos(ctx.size(), cursor_min_length, 0.5, mouse_pos);
                    changed |= self.set_cursor_progress(progress);
                    self.grab_anchor = Some(0.5);
                };
                if changed {
                    ctx.request_render();
                }
            }
            PointerEvent::Move(PointerUpdate { current, .. }) => {
                if ctx.is_active()
                    && let Some(grab_anchor) = self.grab_anchor
                {
                    let cursor_min_length = theme::SCROLLBAR_MIN_SIZE;
                    let progress = self.progress_from_mouse_pos(
                        ctx.size(),
                        cursor_min_length,
                        grab_anchor,
                        ctx.local_position(current.position),
                    );
                    if self.set_cursor_progress(progress) {
                        ctx.request_render();
                    }
                }
            }
            PointerEvent::Up(..) | PointerEvent::Cancel(..) => {
                self.grab_anchor = None;
            }
            _ => {}
        }
    }

    fn on_text_event(
        &mut self,
        ctx: &mut EventCtx<'_>,
        _props: &mut PropertiesMut<'_>,
        event: &TextEvent,
    ) {
        let TextEvent::Keyboard(event) = event else {
            return;
        };
        if event.state != KeyState::Down {
            return;
        }

        // TODO: Remove HACK: Until scale factor rework happens, just pretend it's always 1.0.
        //       https://github.com/linebender/xilem/issues/1264
        let scale = 1.0;
        let line = 120.0 * scale;
        let page = self.portal_size * scale;

        let mut changed = false;
        match (&event.key, self.axis) {
            (Key::Named(NamedKey::ArrowUp), Axis::Vertical) => {
                changed |= self.adjust_by_pixels(-line);
            }
            (Key::Named(NamedKey::ArrowDown), Axis::Vertical) => {
                changed |= self.adjust_by_pixels(line);
            }
            (Key::Named(NamedKey::PageUp), Axis::Vertical) => {
                changed |= self.adjust_by_pixels(-page);
            }
            (Key::Named(NamedKey::PageDown), Axis::Vertical) => {
                changed |= self.adjust_by_pixels(page);
            }
            (Key::Named(NamedKey::Home), Axis::Vertical) => {
                changed |= self.set_cursor_progress(0.0);
            }
            (Key::Named(NamedKey::End), Axis::Vertical) => changed |= self.set_cursor_progress(1.0),
            (Key::Named(NamedKey::ArrowLeft), Axis::Horizontal) => {
                changed |= self.adjust_by_pixels(-line);
            }
            (Key::Named(NamedKey::ArrowRight), Axis::Horizontal) => {
                changed |= self.adjust_by_pixels(line);
            }
            (Key::Named(NamedKey::Home), Axis::Horizontal) => {
                changed |= self.set_cursor_progress(0.0);
            }
            (Key::Named(NamedKey::End), Axis::Horizontal) => {
                changed |= self.set_cursor_progress(1.0);
            }
            _ => {}
        }

        if changed {
            ctx.request_render();
        }
    }

    fn on_access_event(
        &mut self,
        ctx: &mut EventCtx<'_>,
        _props: &mut PropertiesMut<'_>,
        event: &AccessEvent,
    ) {
        if !matches!(
            event.action,
            accesskit::Action::ScrollUp
                | accesskit::Action::ScrollDown
                | accesskit::Action::ScrollLeft
                | accesskit::Action::ScrollRight
        ) {
            return;
        }

        let action_matches_axis = matches!(
            (event.action, self.axis),
            (
                accesskit::Action::ScrollUp | accesskit::Action::ScrollDown,
                Axis::Vertical
            ) | (
                accesskit::Action::ScrollLeft | accesskit::Action::ScrollRight,
                Axis::Horizontal
            )
        );
        if !action_matches_axis {
            return;
        }

        // TODO: Remove HACK: Until scale factor rework happens, just pretend it's always 1.0.
        //       https://github.com/linebender/xilem/issues/1264
        let scale = 1.0;
        let unit = if let Some(accesskit::ActionData::ScrollUnit(unit)) = &event.data {
            *unit
        } else {
            accesskit::ScrollUnit::Item
        };
        let line = 120.0 * scale;
        let page = self.portal_size * scale;
        let amount = match unit {
            accesskit::ScrollUnit::Item => line,
            accesskit::ScrollUnit::Page => page,
        };
        let signed = match event.action {
            accesskit::Action::ScrollUp | accesskit::Action::ScrollLeft => -amount,
            accesskit::Action::ScrollDown | accesskit::Action::ScrollRight => amount,
            _ => 0.0,
        };

        if self.adjust_by_pixels(signed) {
            ctx.request_render();
        }
    }

    fn register_children(&mut self, _ctx: &mut RegisterCtx<'_>) {}

    fn update(
        &mut self,
        _ctx: &mut UpdateCtx<'_>,
        _props: &mut PropertiesMut<'_>,
        _event: &Update,
    ) {
    }

    fn measure(
        &mut self,
        _ctx: &mut MeasureCtx<'_>,
        _props: &PropertiesRef<'_>,
        axis: Axis,
        len_req: LenReq,
        _cross_length: Option<f64>,
    ) -> f64 {
        // TODO: Remove HACK: Until scale factor rework happens, just pretend it's always 1.0.
        //       https://github.com/linebender/xilem/issues/1264
        let scale = 1.0;

        if axis == self.axis {
            // TODO: Consider .max(theme::SCROLLBAR_MIN_SIZE * scale)
            match len_req {
                LenReq::MinContent | LenReq::MaxContent => self.portal_size,
                LenReq::FitContent(space) => space,
            }
        } else {
            let scrollbar_width = theme::SCROLLBAR_WIDTH * scale;
            let cursor_padding = theme::SCROLLBAR_PAD * scale;

            scrollbar_width + cursor_padding * 2.0
        }
    }

    fn layout(&mut self, _ctx: &mut LayoutCtx<'_>, _props: &PropertiesRef<'_>, _size: Size) {}

    fn paint(&mut self, ctx: &mut PaintCtx<'_>, _props: &PropertiesRef<'_>, scene: &mut Scene) {
        let radius = theme::SCROLLBAR_RADIUS;
        let edge_width = theme::SCROLLBAR_EDGE_WIDTH;
        let cursor_padding = theme::SCROLLBAR_PAD;
        let cursor_min_length = theme::SCROLLBAR_MIN_SIZE;

        let (inset_x, inset_y) = self.axis.pack_xy(0.0, cursor_padding);
        let cursor_rect = self
            .cursor_rect(ctx.size(), cursor_min_length)
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
        node: &mut Node,
    ) {
        node.set_orientation(match self.axis {
            Axis::Horizontal => accesskit::Orientation::Horizontal,
            Axis::Vertical => accesskit::Orientation::Vertical,
        });

        let scroll_range = self.scroll_range();
        if scroll_range <= 1e-12 {
            node.clear_scroll_x_min();
            node.clear_scroll_x_max();
            node.clear_scroll_x();
            node.clear_scroll_y_min();
            node.clear_scroll_y_max();
            node.clear_scroll_y();
            node.clear_orientation();
            return;
        }

        let value = (self.cursor_progress.clamp(0.0, 1.0)) * scroll_range;
        match self.axis {
            Axis::Horizontal => {
                node.set_scroll_x_min(0.0);
                node.set_scroll_x_max(scroll_range);
                node.set_scroll_x(value);
                if self.cursor_progress > 1e-12 {
                    node.add_action(accesskit::Action::ScrollLeft);
                }
                if self.cursor_progress + 1e-12 < 1.0 {
                    node.add_action(accesskit::Action::ScrollRight);
                }
            }
            Axis::Vertical => {
                node.set_scroll_y_min(0.0);
                node.set_scroll_y_max(scroll_range);
                node.set_scroll_y(value);
                if self.cursor_progress > 1e-12 {
                    node.add_action(accesskit::Action::ScrollUp);
                }
                if self.cursor_progress + 1e-12 < 1.0 {
                    node.add_action(accesskit::Action::ScrollDown);
                }
            }
        }
    }

    fn children_ids(&self) -> ChildrenIds {
        ChildrenIds::new()
    }

    fn make_trace_span(&self, id: WidgetId) -> Span {
        trace_span!("ScrollBar", id = id.trace())
    }

    fn accepts_focus(&self) -> bool {
        true
    }
}

impl AllowRawMut for ScrollBar {}

// --- MARK: TESTS
#[cfg(test)]
mod tests {
    use super::*;
    use crate::core::TextEvent;
    use crate::core::keyboard::{Key, NamedKey};
    use crate::core::{NewWidget, PointerButton};
    use crate::properties::Dimensions;
    use crate::testing::{TestHarness, assert_render_snapshot};
    use crate::theme::test_property_set;

    #[test]
    fn simple_scrollbar() {
        let widget = NewWidget::new_with_props(
            ScrollBar::new(Axis::Vertical, 200.0, 600.0),
            Dimensions::FIT,
        );

        let mut harness =
            TestHarness::create_with_size(test_property_set(), widget, Size::new(50.0, 200.0));
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
        let widget = NewWidget::new_with_props(
            ScrollBar::new(Axis::Horizontal, 200.0, 600.0),
            Dimensions::FIT,
        );

        let mut harness =
            TestHarness::create_with_size(test_property_set(), widget, Size::new(200.0, 50.0));
        let scrollbar_id = harness.root_id();

        assert_render_snapshot!(harness, "scrollbar_horizontal");

        assert!(harness.pop_action_erased().is_none());

        harness.mouse_click_on(scrollbar_id);
        // TODO - Scroll action?
        assert!(harness.pop_action_erased().is_none());

        assert_render_snapshot!(harness, "scrollbar_horizontal_middle");
    }

    #[test]
    fn keyboard_scroll_updates_access_tree() {
        let widget = NewWidget::new_with_props(
            ScrollBar::new(Axis::Vertical, 200.0, 600.0),
            Dimensions::FIT,
        );
        let mut harness =
            TestHarness::create_with_size(test_property_set(), widget, Size::new(50.0, 200.0));
        let _ = harness.render();

        let scrollbar_id = harness.root_id();
        harness.focus_on(Some(scrollbar_id));

        harness.process_text_event(TextEvent::key_down(Key::Named(NamedKey::ArrowDown)));
        let _ = harness.render();

        let node = harness.access_node(scrollbar_id).unwrap();
        assert!(node.data().scroll_y().unwrap_or(0.0) > 0.0);
    }

    // TODO - Add "portal larger than content" test

    // TODO - Add WidgetMut tests
}
