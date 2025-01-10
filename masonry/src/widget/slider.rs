// Copyright 2023 the Xilem Authors and the Druid Authors
// SPDX-License-Identifier: Apache-2.0

//! A slider widget for selecting a value within a range.

use accesskit::{Node, Role};
use smallvec::SmallVec;
use tracing::{trace_span, Span};
use vello::{kurbo::Affine, Scene};

use crate::{
    theme, AccessCtx, AccessEvent, BoxConstraints, Color, EventCtx, LayoutCtx, PaintCtx,
    PointerEvent, QueryCtx, RegisterCtx, Size, TextEvent, Update, UpdateCtx, Widget,
    WidgetId, Action,
};

use super::WidgetMut;

/// A slider widget for selecting a value within a range.
pub struct Slider {
    value: f64,
    min: f64,
    max: f64,
    color: Color,
    is_dragging: bool,
}

// --- MARK: BUILDERS ---
impl Slider {
    /// Create a new slider with the given range and initial value.
    pub fn new(min: f64, max: f64, value: f64) -> Self {
        Self {
            value: value.clamp(min, max),
            min,
            max,
            color: theme::PRIMARY_LIGHT,
            is_dragging: false,
        }
    }

    /// Builder-style method for setting the slider's color.
    pub fn with_color(mut self, color: impl Into<Color>) -> Self {
        self.color = color.into();
        self
    }
}

// --- MARK: WIDGETMUT ---
impl Slider {
    /// Set the slider's value.
    pub fn set_value(this: &mut WidgetMut<'_, Self>, value: f64) {
        this.widget.value = value.clamp(this.widget.min, this.widget.max);
        this.ctx.request_paint_only();
    }

    /// Set the slider's color.
    pub fn set_color(this: &mut WidgetMut<'_, Self>, color: impl Into<Color>) {
        this.widget.color = color.into();
        this.ctx.request_paint_only();
    }
}

// --- MARK: IMPL WIDGET ---
impl Widget for Slider {
    fn on_pointer_event(&mut self, ctx: &mut EventCtx, event: &PointerEvent) {
        match event {
            PointerEvent::PointerDown(_, state) => {
                if !ctx.is_disabled() {
                    self.is_dragging = true;
                    ctx.capture_pointer();
                    self.update_value(ctx, state.position.x);
                    ctx.request_paint_only();
                }
            }
            PointerEvent::PointerUp(_, _) => {
                if self.is_dragging {
                    self.is_dragging = false;
                    ctx.release_pointer();
                    ctx.request_paint_only();
                }
            }
            PointerEvent::PointerMove(state) => {
                if self.is_dragging {
                    self.update_value(ctx, state.position.x);
                    ctx.request_paint_only();
                }
            }
            _ => (),
        }
    }

    fn on_text_event(&mut self, _ctx: &mut EventCtx, _event: &TextEvent) {}

    fn on_access_event(&mut self, _ctx: &mut EventCtx, _event: &AccessEvent) {}

    fn update(&mut self, ctx: &mut UpdateCtx, event: &Update) {
        match event {
            Update::HoveredChanged(_) | Update::FocusChanged(_) | Update::DisabledChanged(_) => {
                ctx.request_paint_only();
            }
            _ => {}
        }
    }

    fn register_children(&mut self, _ctx: &mut RegisterCtx) {}

    fn layout(&mut self, _ctx: &mut LayoutCtx, bc: &BoxConstraints) -> Size {
        bc.constrain(Size::new(theme::BASIC_WIDGET_HEIGHT * 4.0, theme::BASIC_WIDGET_HEIGHT))
    }

    fn paint(&mut self, ctx: &mut PaintCtx, scene: &mut Scene) {
        let size = ctx.size();
        let width = size.width;
        let height = size.height;

        // Draw the track
        let track_rect = vello::kurbo::Rect::new(0.0, height / 2.0 - 2.0, width, height / 2.0 + 2.0);
        scene.fill(
            vello::peniko::Fill::NonZero,
            Affine::IDENTITY,
            theme::BORDER_DARK,
            None,
            &track_rect,
        );

        // Draw the thumb
        let thumb_x = ((self.value - self.min) / (self.max - self.min)) * width;
        let thumb_rect = vello::kurbo::Rect::new(thumb_x - 5.0, 0.0, thumb_x + 5.0, height);
        scene.fill(
            vello::peniko::Fill::NonZero,
            Affine::IDENTITY,
            self.color,
            None,
            &thumb_rect,
        );
    }

    fn accessibility_role(&self) -> Role {
        Role::Slider
    }

    fn accessibility(&mut self, _ctx: &mut AccessCtx, node: &mut Node) {
        node.set_value(self.value.to_string());
        node.add_action(accesskit::Action::SetValue);
    }

    fn children_ids(&self) -> SmallVec<[WidgetId; 16]> {
        SmallVec::new()
    }

    fn make_trace_span(&self, ctx: &QueryCtx<'_>) -> Span {
        trace_span!("Slider", id = ctx.widget_id().trace())
    }
}

impl Slider {
    fn update_value(&mut self, ctx: &mut EventCtx, x: f64) {
        let width = ctx.size().width;
        let new_value = self.min + (x / width) * (self.max - self.min);
        self.value = new_value.clamp(self.min, self.max);
        ctx.submit_action(Action::SliderValueChanged(self.value));
    }
}

// --- MARK: TESTS ---
#[cfg(test)]
mod tests {
    use super::*;
    use crate::testing::TestHarness;
    use crate::{assert_render_snapshot, palette};

    #[test]
    fn simple_slider() {
        let slider = Slider::new(0.0, 100.0, 50.0);

        let mut harness = TestHarness::create(slider);
        assert_render_snapshot!(harness, "slider_init");

        // 获取 Slider 的 WidgetId
        let slider_id = harness.root_widget().id();

        // 模拟点击 Slider 的某个位置
        harness.mouse_click_on(slider_id);
        assert_render_snapshot!(harness, "slider_clicked");
    }

    #[test]
    fn edit_slider() {
        let image_1 = {
            let slider = Slider::new(0.0, 100.0, 50.0).with_color(palette::css::PURPLE);

            let mut harness = TestHarness::create_with_size(slider, Size::new(200.0, 20.0));
            harness.render()
        };

        let image_2 = {
            let slider = Slider::new(0.0, 100.0, 50.0);

            let mut harness = TestHarness::create_with_size(slider, Size::new(200.0, 20.0));

            // 获取 Slider 的 WidgetId
            let slider_id = harness.root_widget().id();

            harness.edit_widget(slider_id, |mut slider| {
                let mut slider = slider.downcast::<Slider>();
                Slider::set_color(&mut slider, palette::css::PURPLE);
            });

            harness.render()
        };

        // 比较两个图像是否相同
        assert!(image_1 == image_2);
    }
}