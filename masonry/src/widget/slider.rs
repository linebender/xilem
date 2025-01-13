// Copyright 2023 the Xilem Authors and the Druid Authors
// SPDX-License-Identifier: Apache-2.0

//! A slider widget for selecting a value within a range.

use accesskit::{Node, Role};
use cursor_icon::CursorIcon;
use smallvec::SmallVec;
use tracing::{trace_span, Span};
use vello::{
    kurbo::{Affine, Point, Rect, RoundedRect, RoundedRectRadii, Size},
    Scene,
};

use crate::{
    theme, widget::Axis, AccessCtx, AccessEvent, Action, BoxConstraints, Color, EventCtx,
    LayoutCtx, PaintCtx, PointerButton, PointerEvent, QueryCtx, RegisterCtx, TextEvent, Update,
    UpdateCtx, Widget, WidgetId,
};

use super::WidgetMut;

/// A slider widget for selecting a value within a range.
pub struct Slider {
    axis: Axis,
    value: f64,
    min: f64,
    max: f64,
    step: f64,
    color: Color,
    track_color: Color,
    editing: bool,
    is_dragging: bool,
    grab_anchor: Option<f64>,
    thumb_radii: RoundedRectRadii,
    track_radii: RoundedRectRadii,
    is_hovered: bool,
    hover_glow_color: Color,
    hover_glow_blur_radius: f64,
    hover_glow_spread_radius: f64,
    track_rect: Option<RoundedRect>,
}

impl Slider {
    const DEFAULT_WIDTH: f64 = 200.0;
    const DEFAULT_HEIGHT: f64 = 40.0;
    const BASE_TRACK_THICKNESS: f64 = 4.0;
    const TRACK_PADDING: f64 = 20.0;
    const THUMB_WIDTH: f64 = 12.0;
    const THUMB_HEIGHT: f64 = 20.0;

    /// Create a new slider with the given range and initial value.
    pub fn new(axis: Axis, min: f64, max: f64, value: f64) -> Self {
        Self {
            axis,
            value: value.clamp(min, max),
            min,
            max,
            step: 1.0,
            color: theme::PRIMARY_LIGHT,
            track_color: theme::PRIMARY_DARK,
            editing: false,
            is_dragging: false,
            grab_anchor: None,
            thumb_radii: RoundedRectRadii::from_single_radius(5.0),
            track_radii: RoundedRectRadii::from_single_radius(2.0),
            is_hovered: false,
            hover_glow_color: Color::from_rgba8(255, 255, 255, 50),
            hover_glow_blur_radius: 5.0,
            hover_glow_spread_radius: 2.0,
            track_rect: None,
        }
    }

    /// Builder-style method for setting the slider's color.
    pub fn with_color(mut self, color: impl Into<Color>) -> Self {
        self.color = color.into();
        self
    }

    /// Builder-style method for setting the slider's track color.
    pub fn with_track_color(mut self, track_color: impl Into<Color>) -> Self {
        self.track_color = track_color.into();
        self
    }

    /// Builder-style method for setting the slider's step amount.
    pub fn with_step(mut self, step: f64) -> Self {
        self.step = step;
        self
    }

    /// Builder-style method for setting the slider's thumb radii.
    pub fn with_thumb_radii(mut self, radii: impl Into<RoundedRectRadii>) -> Self {
        self.thumb_radii = radii.into();
        self
    }

    /// Builder-style method for setting the slider's track radii.
    pub fn with_track_radii(mut self, radii: impl Into<RoundedRectRadii>) -> Self {
        self.track_radii = radii.into();
        self
    }

    /// Builder-style method for setting the hover glow color.
    pub fn with_hover_glow_color(mut self, color: impl Into<Color>) -> Self {
        self.hover_glow_color = color.into();
        self
    }

    /// Builder-style method for setting the hover glow blur radius.
    pub fn with_hover_glow_blur_radius(mut self, blur_radius: f64) -> Self {
        self.hover_glow_blur_radius = blur_radius;
        self
    }

    /// Builder-style method for setting the hover glow spread radius.
    pub fn with_hover_glow_spread_radius(mut self, spread_radius: f64) -> Self {
        self.hover_glow_spread_radius = spread_radius;
        self
    }
}

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

    /// Set the slider's track color.
    pub fn set_track_color(this: &mut WidgetMut<'_, Self>, track_color: impl Into<Color>) {
        this.widget.track_color = track_color.into();
        this.ctx.request_paint_only();
    }

    /// Set the slider's step amount.
    pub fn set_step(this: &mut WidgetMut<'_, Self>, step: f64) {
        this.widget.step = step;
        this.ctx.request_paint_only();
    }

    /// Set the slider's thumb radii.
    pub fn set_thumb_radii(this: &mut WidgetMut<'_, Self>, radii: impl Into<RoundedRectRadii>) {
        this.widget.thumb_radii = radii.into();
        this.ctx.request_paint_only();
    }

    /// Set the slider's track radii.
    pub fn set_track_radii(this: &mut WidgetMut<'_, Self>, radii: impl Into<RoundedRectRadii>) {
        this.widget.track_radii = radii.into();
        this.ctx.request_paint_only();
    }

    /// Set the hover glow color.
    pub fn set_hover_glow_color(this: &mut WidgetMut<'_, Self>, color: impl Into<Color>) {
        this.widget.hover_glow_color = color.into();
        this.ctx.request_paint_only();
    }

    /// Set the hover glow blur radius.
    pub fn set_hover_glow_blur_radius(this: &mut WidgetMut<'_, Self>, blur_radius: f64) {
        this.widget.hover_glow_blur_radius = blur_radius;
        this.ctx.request_paint_only();
    }

    /// Set the hover glow spread radius.
    pub fn set_hover_glow_spread_radius(this: &mut WidgetMut<'_, Self>, spread_radius: f64) {
        this.widget.hover_glow_spread_radius = spread_radius;
        this.ctx.request_paint_only();
    }
}

impl Widget for Slider {
    fn on_pointer_event(&mut self, ctx: &mut EventCtx, event: &PointerEvent) {
        match event {
            PointerEvent::PointerDown(PointerButton::Primary, state) => {
                if !ctx.is_disabled() {
                    ctx.capture_pointer();
                    let layout_size = ctx.size();
                    let thumb_rect = self.get_thumb_rect(layout_size);

                    let mouse_pos = Point::new(state.position.x, state.position.y)
                        - ctx.window_origin().to_vec2();
                    if thumb_rect.contains(mouse_pos) {
                        let (z0, z1) = self.axis.major_span(thumb_rect);
                        let mouse_major = self.axis.major_pos(mouse_pos);
                        self.grab_anchor = Some((mouse_major - z0) / (z1 - z0));
                    } else {
                        self.value = self.value_from_mouse_pos(layout_size, 0.5, mouse_pos);
                        self.round_to_step();
                        ctx.submit_action(Action::SliderValueChanged(self.value));
                        self.grab_anchor = Some(0.5);
                    }

                    self.is_dragging = true;
                    self.editing = true;
                    ctx.submit_action(Action::SliderEditingChanged(true));
                    ctx.request_paint_only();
                }
            }
            PointerEvent::PointerUp(_, _) => {
                if self.is_dragging {
                    self.is_dragging = false;
                    self.grab_anchor = None;
                    ctx.release_pointer();
                    self.editing = false;
                    ctx.submit_action(Action::SliderEditingChanged(false));
                    ctx.request_paint_only();
                }
            }
            PointerEvent::PointerMove(state) => {
                if self.is_dragging {
                    let mouse_pos = Point::new(state.position.x, state.position.y)
                        - ctx.window_origin().to_vec2();
                    if let Some(grab_anchor) = self.grab_anchor {
                        self.value = self.value_from_mouse_pos(ctx.size(), grab_anchor, mouse_pos);
                        self.round_to_step();
                        ctx.submit_action(Action::SliderValueChanged(self.value));
                    }
                    ctx.request_paint_only();
                } else {
                    let mouse_pos = Point::new(state.position.x, state.position.y)
                        - ctx.window_origin().to_vec2();
                    let thumb_rect = self.get_thumb_rect(ctx.size());
                    let was_hovered = self.is_hovered;
                    self.is_hovered = thumb_rect.contains(mouse_pos);
                    if was_hovered != self.is_hovered {
                        ctx.request_paint_only();
                    }
                }
            }
            PointerEvent::PointerLeave(_) => {
                if self.is_hovered {
                    self.is_hovered = false;
                    ctx.request_paint_only();
                }
            }
            _ => {}
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
        let (width, height) = match self.axis {
            Axis::Horizontal => {
                let width = if bc.is_width_bounded() {
                    bc.max().width
                } else {
                    Self::DEFAULT_WIDTH
                };
                let height = if bc.is_height_bounded() {
                    bc.max().height.min(Self::DEFAULT_HEIGHT)
                } else {
                    Self::DEFAULT_HEIGHT
                };
                (width, height)
            }
            Axis::Vertical => {
                let width = if bc.is_width_bounded() {
                    bc.max().width.min(Self::DEFAULT_HEIGHT)
                } else {
                    Self::DEFAULT_HEIGHT
                };
                let height = if bc.is_height_bounded() {
                    bc.max().height
                } else {
                    Self::DEFAULT_WIDTH
                };
                (width, height)
            }
        };

        let size = bc.constrain(Size::new(width, height));

        // 计算滑轨位置和尺寸
        let track_rect = match self.axis {
            Axis::Horizontal => {
                let y_center = size.height / 2.0;
                Rect::new(
                    Self::TRACK_PADDING,
                    y_center - Self::BASE_TRACK_THICKNESS / 2.0,
                    size.width - Self::TRACK_PADDING,
                    y_center + Self::BASE_TRACK_THICKNESS / 2.0,
                )
            }
            Axis::Vertical => {
                let x_center = size.width / 2.0;
                Rect::new(
                    x_center - Self::BASE_TRACK_THICKNESS / 2.0,
                    Self::TRACK_PADDING,
                    x_center + Self::BASE_TRACK_THICKNESS / 2.0,
                    size.height - Self::TRACK_PADDING,
                )
            }
        }
        .to_rounded_rect(self.track_radii);

        self.track_rect = Some(track_rect);
        size
    }

    fn paint(&mut self, ctx: &mut PaintCtx, scene: &mut Scene) {
        let size = ctx.size();

        if let Some(track_rect) = &self.track_rect {
            scene.fill(
                vello::peniko::Fill::NonZero,
                Affine::IDENTITY,
                self.track_color,
                None,
                track_rect,
            );
        }

        let thumb_rect = self.get_thumb_rect(size);
        if self.is_hovered {
            let glow_rect = thumb_rect
                .inflate(self.hover_glow_spread_radius, self.hover_glow_spread_radius)
                .to_rounded_rect(inflate(
                    &self.thumb_radii,
                    self.hover_glow_spread_radius,
                    self.hover_glow_spread_radius,
                ));
            let rect = thumb_rect.inflate(1.0, 1.0);
            scene.draw_blurred_rounded_rect_in(
                &glow_rect,
                Affine::IDENTITY,
                rect,
                self.hover_glow_color,
                self.thumb_radii.top_left + self.hover_glow_spread_radius,
                self.hover_glow_blur_radius,
            );
        }
        scene.fill(
            vello::peniko::Fill::NonZero,
            Affine::IDENTITY,
            self.color,
            None,
            &thumb_rect.to_rounded_rect(self.thumb_radii),
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

    fn get_debug_text(&self) -> Option<String> {
        Some(self.value.to_string())
    }

    fn get_cursor(&self, _ctx: &QueryCtx, _pos: Point) -> CursorIcon {
        CursorIcon::Text
    }
}

impl Slider {
    fn compute_max_intrinsic(&mut self, _ctx: &mut LayoutCtx, axis: Axis, _cross: f64) -> f64 {
        match (axis, self.axis) {
            (Axis::Horizontal, Axis::Horizontal) => Self::DEFAULT_WIDTH,
            (Axis::Vertical, Axis::Vertical) => Self::DEFAULT_WIDTH,
            (Axis::Horizontal, Axis::Vertical) => Self::DEFAULT_HEIGHT,
            (Axis::Vertical, Axis::Horizontal) => Self::DEFAULT_HEIGHT,
        }
    }

    fn round_to_step(&mut self) {
        self.value = ((self.value - self.min) / self.step).round() * self.step + self.min;
        self.value = self.value.clamp(self.min, self.max);
    }

    fn get_thumb_rect(&self, layout_size: Size) -> Rect {
        let track_rect = self.track_rect.as_ref().expect("track_rect should be set");
        let size_ratio = (self.value - self.min) / (self.max - self.min);

        match self.axis {
            Axis::Horizontal => {
                let x =
                    track_rect.rect().x0 + size_ratio * (track_rect.width() - Self::THUMB_WIDTH);
                let y = layout_size.height / 2.0 - Self::THUMB_HEIGHT / 2.0;
                Rect::from_origin_size(
                    Point::new(x, y),
                    Size::new(Self::THUMB_WIDTH, Self::THUMB_HEIGHT),
                )
            }
            Axis::Vertical => {
                let x = layout_size.width / 2.0 - Self::THUMB_HEIGHT / 2.0;
                let y =
                    track_rect.rect().y0 + size_ratio * (track_rect.height() - Self::THUMB_WIDTH);
                Rect::from_origin_size(
                    Point::new(x, y),
                    Size::new(Self::THUMB_HEIGHT, Self::THUMB_WIDTH),
                )
            }
        }
    }

    fn value_from_mouse_pos(&self, layout_size: Size, anchor: f64, mouse_pos: Point) -> f64 {
        let thumb_rect = self.get_thumb_rect(layout_size);
        let thumb_width = self.axis.major(thumb_rect.size());
        let new_thumb_pos_major = self.axis.major_pos(mouse_pos) - anchor * thumb_width;

        let track_rect = self.track_rect.as_ref().expect("track_rect should be set");
        let track_length = match self.axis {
            Axis::Horizontal => track_rect.width(),
            Axis::Vertical => track_rect.height(),
        };

        let track_pos = match self.axis {
            Axis::Horizontal => track_rect.rect().x0,
            Axis::Vertical => track_rect.rect().y0,
        };

        let normalized_pos = (new_thumb_pos_major - track_pos) / track_length;
        let new_value = self.min + normalized_pos * (self.max - self.min);
        new_value.clamp(self.min, self.max)
    }
}

/// Inflate the radii by the given amounts.
pub fn inflate(raddi: &RoundedRectRadii, width: f64, height: f64) -> RoundedRectRadii {
    RoundedRectRadii::new(
        raddi.top_left + width,
        raddi.top_right + width,
        raddi.bottom_right + height,
        raddi.bottom_left + height,
    )
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::palette;
    use crate::testing::TestHarness;

    #[test]
    fn test_slider_drag() {
        let slider = Slider::new(Axis::Horizontal, 0.0, 100.0, 50.0)
            .with_color(palette::css::BLUE)
            .with_track_color(palette::css::LIGHT_GRAY);

        let mut harness = TestHarness::create(slider);
        let slider_id = harness.root_widget().id();

        harness.mouse_click_on(slider_id);
        assert_eq!(
            harness
                .get_widget(slider_id)
                .downcast::<Slider>()
                .unwrap()
                .value,
            50.0
        );

        harness.mouse_move(Point::new(150.0, 10.0));
        assert_eq!(
            harness
                .get_widget(slider_id)
                .downcast::<Slider>()
                .unwrap()
                .value,
            75.0
        );

        harness.mouse_move(Point::new(50.0, 10.0));
        assert_eq!(
            harness
                .get_widget(slider_id)
                .downcast::<Slider>()
                .unwrap()
                .value,
            25.0
        );
    }

    #[test]
    fn test_slider_step() {
        let slider = Slider::new(Axis::Horizontal, 0.0, 100.0, 50.0)
            .with_step(10.0)
            .with_color(palette::css::BLUE)
            .with_track_color(palette::css::LIGHT_GRAY);

        let mut harness = TestHarness::create(slider);
        let slider_id = harness.root_widget().id();

        harness.mouse_click_on(slider_id);
        harness.mouse_move(Point::new(150.0, 10.0));
        assert_eq!(
            harness
                .get_widget(slider_id)
                .downcast::<Slider>()
                .unwrap()
                .value,
            80.0
        );
    }

    #[test]
    fn test_slider_bounds() {
        let slider = Slider::new(Axis::Horizontal, 0.0, 100.0, 50.0)
            .with_color(palette::css::BLUE)
            .with_track_color(palette::css::LIGHT_GRAY);

        let mut harness = TestHarness::create(slider);
        let slider_id = harness.root_widget().id();

        harness.mouse_click_on(slider_id);

        // Test upper bound
        harness.mouse_move(Point::new(1000.0, 10.0));
        assert_eq!(
            harness
                .get_widget(slider_id)
                .downcast::<Slider>()
                .unwrap()
                .value,
            100.0
        );

        // Test lower bound
        harness.mouse_move(Point::new(-1000.0, 10.0));
        assert_eq!(
            harness
                .get_widget(slider_id)
                .downcast::<Slider>()
                .unwrap()
                .value,
            0.0
        );
    }
}
