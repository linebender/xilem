// This software is licensed under Apache License 2.0 and distributed on an
// "as-is" basis without warranties of any kind. See the LICENSE file for
// details.

#![allow(missing_docs)]
#![allow(unused)]

use druid_shell::kurbo::Rect;
use smallvec::SmallVec;
use tracing::{trace_span, Span};

use super::Axis;
use crate::widget::WidgetRef;
use crate::{
    theme, BoxConstraints, Env, Event, EventCtx, LayoutCtx, LifeCycle, LifeCycleCtx, PaintCtx,
    Point, RenderContext, Selector, Size, StatusChange, Widget,
};

// RULES
// -

// TODO - Document names:
// - grabbybar
// - empty_space
// - _z
// - _length

// TODO - Fade scrollbars? Find out how Linux/MacOS/Windows do it
// TODO - Rename cursor to oval/rect/bar/grabber/grabbybar
// TODO - Rename progress to ???
pub struct ScrollBar {
    axis: Axis,
    pub portal_size: f64,
    pub content_size: f64,
    cursor_progress: f64,
    hovered: bool,
    grab_anchor: Option<f64>,
}

crate::declare_widget!(ScrollBarMut, ScrollBar);

pub const SCROLLBAR_MOVED: Selector<(Axis, f64)> = Selector::new("masonry-builtin.scrollbar-moved");

impl ScrollBar {
    pub fn new(axis: Axis, portal_size: f64, content_size: f64) -> Self {
        Self {
            axis,
            portal_size,
            content_size,
            cursor_progress: 0.0,
            hovered: false,
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

        let mouse_pos_major = self.axis.major_pos(mouse_pos);

        // invariant: cursor_x == progress * (1 - size_ratio) * layout_width
        // invariant: cursor_x + anchor * cursor_width == mouse_x

        let cursor_width = self.axis.major(cursor_rect.size());
        let new_cursor_pos_major = self.axis.major_pos(mouse_pos) - anchor * cursor_width;

        let empty_space_length = (1.0 - size_ratio) * self.axis.major(layout_size);
        let new_cursor_progress = new_cursor_pos_major / empty_space_length;

        new_cursor_progress.clamp(0.0, 1.0)
    }
}

impl ScrollBarMut<'_, '_> {
    pub fn set_sizes(&mut self, portal_size: f64, content_size: f64) {
        self.widget.portal_size = portal_size;
        self.widget.content_size = content_size;
        self.ctx.request_paint();
    }

    pub fn set_content_size(&mut self, content_size: f64) {
        // TODO - cursor_progress
        self.widget.content_size = content_size;
        self.ctx.request_paint();
    }

    pub fn set_cursor_progress(&mut self, cursor_progress: f64) {
        self.widget.cursor_progress = cursor_progress;
        self.ctx.request_paint();
    }
}

// --- TRAIT IMPLS ---

impl Widget for ScrollBar {
    fn on_event(&mut self, ctx: &mut EventCtx, event: &Event, env: &Env) {
        match event {
            Event::MouseDown(event) => {
                ctx.set_active(true);

                let cursor_min_length = env.get(theme::SCROLLBAR_MIN_SIZE);
                let cursor_rect = self.get_cursor_rect(ctx.size(), cursor_min_length);

                if cursor_rect.contains(event.pos) {
                    let (z0, z1) = self.axis.major_span(cursor_rect);
                    let mouse_major = self.axis.major_pos(event.pos);
                    self.grab_anchor = Some((mouse_major - z0) / (z1 - z0));
                } else {
                    self.cursor_progress =
                        self.progress_from_mouse_pos(ctx.size(), cursor_min_length, 0.5, event.pos);
                    ctx.submit_notification(
                        SCROLLBAR_MOVED.with((self.axis, self.cursor_progress)),
                    );
                    self.grab_anchor = Some(0.5);
                };
                ctx.request_paint();
            }
            Event::MouseMove(event) => {
                if let Some(grab_anchor) = self.grab_anchor {
                    let cursor_min_length = env.get(theme::SCROLLBAR_MIN_SIZE);
                    self.cursor_progress = self.progress_from_mouse_pos(
                        ctx.size(),
                        cursor_min_length,
                        grab_anchor,
                        event.pos,
                    );
                    ctx.submit_notification(
                        SCROLLBAR_MOVED.with((self.axis, self.cursor_progress)),
                    );
                }
                ctx.request_paint();
            }
            Event::MouseUp(event) => {
                self.grab_anchor = None;
                ctx.set_active(false);
                ctx.request_paint();
            }
            _ => {}
        }
    }

    fn on_status_change(&mut self, ctx: &mut LifeCycleCtx, event: &StatusChange, env: &Env) {}

    fn lifecycle(&mut self, ctx: &mut LifeCycleCtx, event: &LifeCycle, env: &Env) {}

    fn layout(&mut self, ctx: &mut LayoutCtx, bc: &BoxConstraints, env: &Env) -> Size {
        // TODO - handle resize

        let scrollbar_width = env.get(theme::SCROLLBAR_WIDTH);
        let cursor_padding = env.get(theme::SCROLLBAR_PAD);
        self.axis
            .pack(
                self.axis.major(bc.max()),
                scrollbar_width + cursor_padding * 2.0,
            )
            .into()
    }

    fn paint(&mut self, ctx: &mut PaintCtx, env: &Env) {
        let brush = ctx.render_ctx.solid_brush(env.get(theme::SCROLLBAR_COLOR));
        let border_brush = ctx
            .render_ctx
            .solid_brush(env.get(theme::SCROLLBAR_BORDER_COLOR));

        let radius = env.get(theme::SCROLLBAR_RADIUS);
        let edge_width = env.get(theme::SCROLLBAR_EDGE_WIDTH);
        let cursor_padding = env.get(theme::SCROLLBAR_PAD);
        let cursor_min_length = env.get(theme::SCROLLBAR_MIN_SIZE);

        let (inset_x, inset_y) = self.axis.pack(0.0, cursor_padding);
        let cursor_rect = self
            .get_cursor_rect(ctx.size(), cursor_min_length)
            .inset((-inset_x, -inset_y))
            .to_rounded_rect(radius);
        ctx.render_ctx.fill(cursor_rect, &brush);
        ctx.render_ctx
            .stroke(cursor_rect, &border_brush, edge_width);
    }

    fn children(&self) -> SmallVec<[WidgetRef<'_, dyn Widget>; 16]> {
        SmallVec::new()
    }

    fn make_trace_span(&self) -> Span {
        trace_span!("ScrollBar")
    }
}

#[cfg(test)]
mod tests {
    use druid_shell::MouseButton;
    use insta::assert_debug_snapshot;

    use super::*;
    use crate::assert_render_snapshot;
    use crate::testing::{widget_ids, TestHarness, TestWidgetExt};

    #[test]
    fn simple_scrollbar() {
        let [scrollbar_id] = widget_ids();
        let widget = ScrollBar::new(Axis::Vertical, 200.0, 600.0).with_id(scrollbar_id);

        let mut harness = TestHarness::create_with_size(widget, Size::new(50.0, 200.0));

        assert_debug_snapshot!(harness.root_widget());
        assert_render_snapshot!(harness, "scrollbar_default");

        assert_eq!(harness.pop_action(), None);

        harness.mouse_click_on(scrollbar_id);
        // TODO - Scroll action?
        assert_eq!(harness.pop_action(), None);

        assert_render_snapshot!(harness, "scrollbar_middle");

        harness.mouse_button_press(MouseButton::Left);
        harness.mouse_move(Point::new(30.0, 150.0));

        assert_render_snapshot!(harness, "scrollbar_down");

        harness.mouse_move(Point::new(30.0, 300.0));
        assert_render_snapshot!(harness, "scrollbar_bottom");
    }

    #[test]
    fn horizontal_scrollbar() {
        let [scrollbar_id] = widget_ids();
        let widget = ScrollBar::new(Axis::Horizontal, 200.0, 600.0).with_id(scrollbar_id);

        let mut harness = TestHarness::create_with_size(widget, Size::new(200.0, 50.0));

        assert_debug_snapshot!(harness.root_widget());
        assert_render_snapshot!(harness, "scrollbar_horizontal");

        assert_eq!(harness.pop_action(), None);

        harness.mouse_click_on(scrollbar_id);
        // TODO - Scroll action?
        assert_eq!(harness.pop_action(), None);

        assert_render_snapshot!(harness, "scrollbar_horizontal_middle");
    }

    // TODO - portal larger than content

    #[cfg(FALSE)]
    #[test]
    fn edit_button() {
        let image_1 = {
            let button = Button::from_label(
                Label::new("The quick brown fox jumps over the lazy dog")
                    .with_text_color(PRIMARY_LIGHT)
                    .with_text_size(20.0),
            );

            let mut harness = TestHarness::create_with_size(button, Size::new(50.0, 50.0));

            harness.render()
        };

        let image_2 = {
            let button = Button::new("Hello world");

            let mut harness = TestHarness::create_with_size(button, Size::new(50.0, 50.0));

            harness.edit_root_widget(|mut button, _| {
                let mut button = button.downcast::<Button>().unwrap();
                button.set_text("The quick brown fox jumps over the lazy dog");

                let mut label = button.label_mut();
                label.set_text_color(PRIMARY_LIGHT);
                label.set_text_size(20.0);
            });

            harness.render()
        };

        // We don't use assert_eq because we don't want rich assert
        assert!(image_1 == image_2);
    }
}
