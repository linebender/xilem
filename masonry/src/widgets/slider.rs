// Copyright 2023 the Xilem Authors and the Druid Authors
// SPDX-License-Identifier: Apache-2.0

use accesskit::{Node, Role};
use cursor_icon::CursorIcon;
use smallvec::SmallVec;
use tracing::{trace_span, Span};
use vello::{
    kurbo::{Affine, Point, Rect, RoundedRect, RoundedRectRadii, Shape as _, Size},
    peniko::{Brush, Color},
    Scene,
};

use crate::{
    core::{
        AccessCtx, AccessEvent, Action, BoxConstraints, EventCtx, LayoutCtx, PaintCtx,
        PointerButton, PointerEvent, QueryCtx, RegisterCtx, TextEvent, Update, UpdateCtx, Widget,
        WidgetId, WidgetMut,
    },
    theme,
};

use super::Axis;

/// A slider widget for selecting a value within a range.
pub struct Slider {
    axis: Axis,
    value: f64,
    min: f64,
    max: f64,
    step: f64,
    thumb_color: Brush,
    track_color: Brush,
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

// Constructor and methods
impl Slider {
    /// Creates a new slider with the specified range and initial value.
    pub fn new(axis: Axis, min: f64, max: f64, value: f64) -> Self {
        debug_assert!(
            max >= min,
            "Slider max value must be greater than or equal to min value"
        );
        let (min, max) = (min.min(max), min.max(max));
        Self {
            axis,
            value: value.clamp(min, max),
            min,
            max,
            step: theme::SLIDER_STEP,
            thumb_color: theme::PRIMARY_LIGHT.into(),
            track_color: theme::PRIMARY_DARK.into(),
            editing: false,
            is_dragging: false,
            grab_anchor: None,
            thumb_radii: RoundedRectRadii::from_single_radius(theme::SLIDER_THUMB_RADIUS),
            track_radii: RoundedRectRadii::from_single_radius(theme::SLIDER_TRACK_RADIUS),
            is_hovered: false,
            hover_glow_color: theme::SLIDER_HOVER_GLOW_COLOR,
            hover_glow_blur_radius: theme::SLIDER_HOVER_GLOW_BLUR_RADIUS,
            hover_glow_spread_radius: theme::SLIDER_HOVER_GLOW_SPREAD_RADIUS,
            track_rect: None,
        }
    }

    /// Sets the slider's color.
    pub fn with_thumb_color(mut self, color: impl Into<Brush>) -> Self {
        self.thumb_color = color.into();
        self
    }

    /// Sets the slider's track color.
    pub fn with_track_color(mut self, track_color: impl Into<Brush>) -> Self {
        self.track_color = track_color.into();
        self
    }

    /// Sets the slider's step size.
    pub fn with_step(mut self, step: f64) -> Self {
        self.step = step;
        self
    }

    /// Sets the slider's thumb radii.
    pub fn with_thumb_radii(mut self, radii: impl Into<RoundedRectRadii>) -> Self {
        self.thumb_radii = radii.into();
        self
    }

    /// Sets the slider's track radii.
    pub fn with_track_radii(mut self, radii: impl Into<RoundedRectRadii>) -> Self {
        self.track_radii = radii.into();
        self
    }

    /// Sets the hover glow color.
    pub fn with_hover_glow_color(mut self, color: impl Into<Color>) -> Self {
        self.hover_glow_color = color.into();
        self
    }

    /// Sets the hover glow blur radius.
    pub fn with_hover_glow_blur_radius(mut self, blur_radius: f64) -> Self {
        self.hover_glow_blur_radius = blur_radius;
        self
    }

    /// Sets the hover glow spread radius.
    pub fn with_hover_glow_spread_radius(mut self, spread_radius: f64) -> Self {
        self.hover_glow_spread_radius = spread_radius;
        self
    }

    /// Gets the current value ratio (0.0-1.0) based on the slider's value.
    fn value_ratio(&self) -> f64 {
        if (self.max - self.min).abs() < f64::EPSILON {
            0.0
        } else {
            (self.value - self.min) / (self.max - self.min)
        }
    }

    /// Calculates the thumb rectangle based on the current layout size.
    fn thumb_rect(&self, layout_size: Size) -> RoundedRect {
        let track_rect = self
            .track_rect
            .expect("Track rect should be calculated during layout");
        let ratio = self.value_ratio();

        let (pos, size) = match self.axis {
            Axis::Horizontal => {
                let x =
                    track_rect.rect().x0 + ratio * (track_rect.width() - theme::SLIDER_THUMB_WIDTH);
                let y = (layout_size.height - theme::SLIDER_THUMB_HEIGHT) / 2.0;
                (
                    Point::new(x, y),
                    Size::new(theme::SLIDER_THUMB_WIDTH, theme::SLIDER_THUMB_HEIGHT),
                )
            }
            Axis::Vertical => {
                // Vertical axis needs reversed coordinates (0.0 at bottom, 1.0 at top)
                let reversed_ratio = 1.0 - ratio;
                let x = (layout_size.width - theme::SLIDER_THUMB_HEIGHT) / 2.0; // Swap width and height
                let y = track_rect.rect().y0
                    + reversed_ratio * (track_rect.height() - theme::SLIDER_THUMB_WIDTH);
                (
                    Point::new(x, y),
                    Size::new(theme::SLIDER_THUMB_HEIGHT, theme::SLIDER_THUMB_WIDTH), // Swap width and height
                )
            }
        };

        Rect::from_origin_size(pos, size).to_rounded_rect(self.thumb_radii)
    }

    /// Rounds the current value to the nearest step.
    fn round_to_step(&mut self) {
        if self.step <= 0.0 {
            return;
        }
        self.value = ((self.value - self.min) / self.step).round() * self.step + self.min;
        self.value = self.value.clamp(self.min, self.max);
    }

    /// Updates the slider's value based on the mouse position.
    fn update_value_from_pos(&mut self, mouse_pos: Point, ctx: &mut EventCtx) {
        if let Some(anchor) = self.grab_anchor {
            let track_rect = self.track_rect.expect("Track rect should be set");
            let thumb_size = match self.axis {
                Axis::Horizontal => theme::SLIDER_THUMB_WIDTH,
                Axis::Vertical => theme::SLIDER_THUMB_WIDTH, // Use swapped width
            };

            let mouse_major = self.axis.major_pos(mouse_pos);
            let track_start = self.axis.major_pos(track_rect.rect().origin());
            let track_length = self.axis.major(track_rect.rect().size()) - thumb_size;

            let new_pos =
                (mouse_major - track_start - anchor * thumb_size).clamp(0.0, track_length);
            let ratio = match self.axis {
                Axis::Horizontal => new_pos / track_length,
                Axis::Vertical => 1.0 - (new_pos / track_length), // Reverse ratio
            };

            self.value = self.min + ratio * (self.max - self.min);
            self.round_to_step();
            ctx.submit_action(Action::SliderValueChanged(self.value));
        }
    }
}

impl Slider {
    /// Sets the slider's value.
    pub fn set_value(this: &mut WidgetMut<'_, Self>, value: f64) {
        this.widget.value = value.clamp(this.widget.min, this.widget.max);
        this.ctx.request_paint_only();
    }

    /// Sets the slider's minimum value.
    pub fn set_min(this: &mut WidgetMut<'_, Self>, min: f64) {
        this.widget.min = min;
        this.widget.value = this.widget.value.clamp(this.widget.min, this.widget.max);
        this.ctx.request_paint_only();
    }

    /// Sets the slider's maximum value.
    pub fn set_max(this: &mut WidgetMut<'_, Self>, max: f64) {
        this.widget.max = max;
        this.widget.value = this.widget.value.clamp(this.widget.min, this.widget.max);
        this.ctx.request_paint_only();
    }

    /// Sets the slider's axis.
    pub fn set_axis(this: &mut WidgetMut<'_, Self>, axis: Axis) {
        this.widget.axis = axis;
        this.ctx.request_layout();
        this.ctx.request_paint_only();
    }

    /// Sets the slider's color.
    pub fn set_thumb_color(this: &mut WidgetMut<'_, Self>, color: impl Into<Brush>) {
        this.widget.thumb_color = color.into();
        this.ctx.request_paint_only();
    }

    /// Sets the slider's track color.
    pub fn set_track_color(this: &mut WidgetMut<'_, Self>, track_color: impl Into<Brush>) {
        this.widget.track_color = track_color.into();
        this.ctx.request_paint_only();
    }

    /// Sets the slider's step size.
    pub fn set_step(this: &mut WidgetMut<'_, Self>, step: f64) {
        this.widget.step = step;
        this.ctx.request_paint_only();
    }

    /// Sets the slider's thumb radii.
    pub fn set_thumb_radii(this: &mut WidgetMut<'_, Self>, radii: impl Into<RoundedRectRadii>) {
        this.widget.thumb_radii = radii.into();
        this.ctx.request_paint_only();
    }

    /// Sets the slider's track radii.
    pub fn set_track_radii(this: &mut WidgetMut<'_, Self>, radii: impl Into<RoundedRectRadii>) {
        this.widget.track_radii = radii.into();
        this.ctx.request_paint_only();
    }

    /// Sets the hover glow color.
    pub fn set_hover_glow_color(this: &mut WidgetMut<'_, Self>, color: impl Into<Color>) {
        this.widget.hover_glow_color = color.into();
        this.ctx.request_paint_only();
    }

    /// Sets the hover glow blur radius.
    pub fn set_hover_glow_blur_radius(this: &mut WidgetMut<'_, Self>, blur_radius: f64) {
        this.widget.hover_glow_blur_radius = blur_radius;
        this.ctx.request_paint_only();
    }

    /// Sets the hover glow spread radius.
    pub fn set_hover_glow_spread_radius(this: &mut WidgetMut<'_, Self>, spread_radius: f64) {
        this.widget.hover_glow_spread_radius = spread_radius;
        this.ctx.request_paint_only();
    }
}

impl Widget for Slider {
    fn on_pointer_event(&mut self, ctx: &mut EventCtx, event: &PointerEvent) {
        match event {
            PointerEvent::PointerDown(PointerButton::Primary, _) if !ctx.is_disabled() => {
                ctx.capture_pointer();
                let thumb_rect = self.thumb_rect(ctx.size());
                let mouse_pos = event.local_position(ctx);

                if thumb_rect.contains(mouse_pos) {
                    let bounds = thumb_rect.rect();
                    let local_pos = mouse_pos - bounds.origin();
                    self.grab_anchor = Some(match self.axis {
                        Axis::Horizontal => local_pos.x / bounds.width(),
                        Axis::Vertical => local_pos.y / bounds.height(),
                    });
                } else {
                    self.grab_anchor = Some(0.5);
                    self.update_value_from_pos(mouse_pos, ctx);
                }

                self.is_dragging = true;
                self.editing = true;
                ctx.submit_action(Action::SliderEditingChanged(true));
                ctx.request_paint_only();
            }

            PointerEvent::PointerUp(_, _) if self.is_dragging => {
                self.is_dragging = false;
                self.grab_anchor = None;
                ctx.release_pointer();
                self.editing = false;
                ctx.submit_action(Action::SliderEditingChanged(false));
                ctx.request_paint_only();
            }

            PointerEvent::PointerMove(_) if self.is_dragging => {
                let mouse_pos = event.local_position(ctx);
                self.update_value_from_pos(mouse_pos, ctx);
                ctx.request_paint_only();
            }

            PointerEvent::PointerMove(_) => {
                let thumb_rect = self.thumb_rect(ctx.size());
                let mouse_pos = event.local_position(ctx);
                let hovered = thumb_rect.contains(mouse_pos);
                if hovered != self.is_hovered {
                    self.is_hovered = hovered;
                    ctx.request_paint_only();
                }
            }

            PointerEvent::PointerLeave(_) if self.is_hovered => {
                self.is_hovered = false;
                ctx.request_paint_only();
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

    fn layout(&mut self, _: &mut LayoutCtx, bc: &BoxConstraints) -> Size {
        let (main, cross) = match self.axis {
            Axis::Horizontal => (
                bc.max().width.min(theme::SLIDER_DEFAULT_WIDTH),
                bc.max().height.min(theme::SLIDER_DEFAULT_HEIGHT),
            ),
            Axis::Vertical => (
                bc.max().height.min(theme::SLIDER_DEFAULT_HEIGHT),
                bc.max().width.min(theme::SLIDER_DEFAULT_WIDTH),
            ),
        };

        let size = bc.constrain(Size::new(main, cross));

        // Calculate track position (adjusted for padding)
        let track_rect = match self.axis {
            Axis::Horizontal => Rect::new(
                theme::SLIDER_TRACK_PADDING,
                (size.height - theme::SLIDER_BASE_TRACK_THICKNESS) / 2.0,
                size.width - theme::SLIDER_TRACK_PADDING,
                (size.height + theme::SLIDER_BASE_TRACK_THICKNESS) / 2.0,
            ),
            Axis::Vertical => Rect::new(
                (size.width - theme::SLIDER_BASE_TRACK_THICKNESS) / 2.0,
                theme::SLIDER_TRACK_PADDING,
                (size.width + theme::SLIDER_BASE_TRACK_THICKNESS) / 2.0,
                size.height - theme::SLIDER_TRACK_PADDING,
            ),
        }
        .to_rounded_rect(self.track_radii);

        self.track_rect = Some(track_rect);
        size
    }

    fn paint(&mut self, ctx: &mut PaintCtx, scene: &mut Scene) {
        if let Some(track) = &self.track_rect {
            scene.fill(
                vello::peniko::Fill::NonZero,
                Affine::IDENTITY,
                &self.track_color,
                None,
                track,
            );
        }

        // Draw the thumb
        let thumb = self.thumb_rect(ctx.size());
        scene.fill(
            vello::peniko::Fill::NonZero,
            Affine::IDENTITY,
            &self.thumb_color,
            None,
            &thumb,
        );

        if self.is_hovered {
            scene.draw_blurred_rounded_rect(
                Affine::IDENTITY,
                thumb.rect(),
                self.hover_glow_color,
                self.thumb_radii.top_left + self.hover_glow_spread_radius,
                self.hover_glow_blur_radius,
            );
        }
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

    fn get_cursor(&self, ctx: &QueryCtx, _pos: Point) -> CursorIcon {
        if ctx.is_disabled() {
            CursorIcon::Wait
        } else if self.is_dragging {
            CursorIcon::Grabbing
        } else if self.is_hovered {
            CursorIcon::Grab
        } else {
            CursorIcon::Default
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    use crate::{
        assert_render_snapshot,
        core::Action,
        testing::{widget_ids, TestHarness, TestWidgetExt},
    };
    use insta::assert_debug_snapshot;

    const TEST_SIZE: Size = Size::new(200.0, 40.0); // Standard test size

    #[test]
    fn horizontal_slider_default() {
        let [slider_id] = widget_ids();
        let slider = Slider::new(Axis::Horizontal, 0.0, 100.0, 50.0).with_id(slider_id);

        let mut harness = TestHarness::create_with_size(slider, TEST_SIZE);

        let widget = harness
            .get_widget(slider_id)
            .downcast::<Slider>()
            .expect("Slider widget not found");
        assert_eq!(widget.value, 50.0);
        assert!(!widget.editing);
        assert_debug_snapshot!(harness.root_widget());
        assert_render_snapshot!(harness, "horizontal_slider_default");
    }

    #[test]
    fn vertical_slider() {
        let [slider_id] = widget_ids();
        let slider = Slider::new(Axis::Vertical, 0.0, 100.0, 75.0).with_id(slider_id);

        let mut harness = TestHarness::create_with_size(slider, Size::new(40.0, 200.0));
        assert_render_snapshot!(harness, "vertical_slider");
    }

    #[test]
    fn slider_min_value() {
        let [slider_id] = widget_ids();
        let slider = Slider::new(Axis::Horizontal, 0.0, 100.0, 0.0).with_id(slider_id);

        let mut harness = TestHarness::create_with_size(slider, TEST_SIZE);
        assert_render_snapshot!(harness, "slider_min_value");
    }

    #[test]
    fn slider_max_value() {
        let [slider_id] = widget_ids();
        let slider = Slider::new(Axis::Horizontal, 0.0, 100.0, 100.0).with_id(slider_id);

        let mut harness = TestHarness::create_with_size(slider, TEST_SIZE);
        assert_render_snapshot!(harness, "slider_max_value");
    }

    #[test]
    fn slider_custom_colors() {
        let [slider_id] = widget_ids();
        let slider = Slider::new(Axis::Horizontal, 0.0, 100.0, 30.0)
            .with_thumb_color(Color::from_rgb8(255, 0, 0))
            .with_track_color(Color::from_rgb8(0, 255, 0))
            .with_id(slider_id);

        let mut harness = TestHarness::create_with_size(slider, TEST_SIZE);
        assert_render_snapshot!(harness, "slider_custom_colors");
    }

    #[test]
    fn slider_step_adjustment() {
        let [slider_id] = widget_ids();
        let slider = Slider::new(Axis::Horizontal, 0.0, 100.0, 50.0)
            .with_step(10.0)
            .with_id(slider_id);

        let mut harness = TestHarness::create_with_size(slider, TEST_SIZE);

        /*****************************************************************
         * Phase 1: Mouse down and drag to 84px (step adjustment verification)
         *
         * Calculation details:
         * Track length: 200 - 2*5 = 190
         * Movable range: 190 - 12 = 178
         * Mouse position: 84px (relative to window)
         * Relative track position: 84 - 5 = 79
         * Anchor offset: 79 - 6 = 73
         * Ratio: 73/178 ≈ 0.4101
         * Calculated value: 0.4101 * 100 = 41.01 → rounded to 40
         *****************************************************************/
        harness.mouse_move_to(slider_id);
        harness.mouse_button_press(PointerButton::Primary);
        assert_eq!(
            harness.pop_action(),
            Some((Action::SliderEditingChanged(true), slider_id)),
            "Should trigger editing start on press"
        );

        harness.mouse_move(Point::new(84.0, 20.0));
        assert_eq!(
            harness.pop_action(),
            Some((Action::SliderValueChanged(40.0), slider_id)),
            "Moving to 84px should trigger value change to 40"
        );

        harness.mouse_button_release(PointerButton::Primary);
        assert_eq!(
            harness.pop_action(),
            Some((Action::SliderEditingChanged(false), slider_id)),
            "Should end editing on release"
        );

        // Verify final value
        let widget = harness.get_widget(slider_id).downcast::<Slider>().unwrap();
        assert_eq!(widget.value, 40.0, "Final value should be 40");
        assert!(harness.pop_action().is_none(), "No unhandled actions");
    }

    #[test]
    fn slider_edge_cases() {
        let [slider_id] = widget_ids();
        let slider = Slider::new(Axis::Horizontal, 50.0, 50.0, 50.0).with_id(slider_id);

        let mut harness = TestHarness::create_with_size(slider, TEST_SIZE);

        /*****************************************************************
         * Phase 1: Attempt to drag when min == max
         *****************************************************************/
        harness.mouse_move_to(slider_id);
        assert!(harness.pop_action().is_none(), "No unhandled actions");
        harness.mouse_button_press(PointerButton::Primary);
        assert_eq!(
            harness.pop_action(),
            Some((Action::SliderValueChanged(50.0), slider_id)),
            "Value should remain 50 when min == max"
        );
        assert_eq!(
            harness.pop_action(),
            Some((Action::SliderEditingChanged(true), slider_id)),
            "Should trigger editing start on press"
        );

        harness.mouse_move(Point::new(80.0, 20.0));
        assert_eq!(
            harness.pop_action(),
            Some((Action::SliderValueChanged(50.0), slider_id)),
            "Value should remain 50 when min == max"
        );

        harness.mouse_button_release(PointerButton::Primary);
        assert_eq!(
            harness.pop_action(),
            Some((Action::SliderEditingChanged(false), slider_id)),
            "Should end editing on release"
        );

        // Verify final value
        let widget = harness.get_widget(slider_id).downcast::<Slider>().unwrap();
        assert_eq!(widget.value, 50.0, "Value should remain 50");
        assert!(harness.pop_action().is_none(), "No unhandled actions");
    }

    /// [Layout diagram]
    /// |←5→[====================Track 190px====================]←5→|
    ///        |←12→Thumb→|                         |←12→|
    ///
    /// Click position:
    /// |←5→[············●····································]←5→|
    ///           150px absolute position
    ///           Converted: 150 - 5 = 145 (relative to track start)
    ///           Subtract half thumb width: 145 - 6 = 139
    ///           Effective ratio: 139/178 ≈ 78%
    #[test]
    fn slider_interaction_flow() {
        let [slider_id] = widget_ids();
        let slider = Slider::new(Axis::Horizontal, 0.0, 100.0, 50.0)
            .with_step(1.0)
            .with_id(slider_id);

        let mut harness = TestHarness::create_with_size(slider, TEST_SIZE);

        harness.mouse_move_to(slider_id);
        assert!(harness.pop_action().is_none());

        /*****************************************************************
         * Phase 1: Mouse down event (start dragging)
         *****************************************************************/
        harness.mouse_button_press(PointerButton::Primary);
        assert_eq!(
            harness.pop_action(),
            Some((Action::SliderEditingChanged(true), slider_id))
        );

        /*****************************************************************
         * Phase 2: Mouse move to 75% position calculation
         *
         * Known parameters:
         * - Total width Slider::DEFAULT_WIDTH = 200.0
         * - Track padding TRACK_PADDING = 5.0
         * - Thumb width THUMB_WIDTH = 12.0
         * - Test click position: 75% of total width (200 * 0.75 = 150.0)
         *
         * Calculation steps:
         * 1. Track available range
         *    track_rect.width() = DEFAULT_WIDTH - 2*TRACK_PADDING
         *                      = 200 - 2*5 = 190.0
         *
         * 2. Thumb movable range
         *    track_length = track_rect.width() - THUMB_WIDTH
         *                 = 190 - 12 = 178.0
         *
         * 3. Mouse coordinate conversion (relative to track start)
         *    mouse_major = 150.0 (horizontal click position)
         *    track_start = TRACK_PADDING = 5.0
         *
         * 4. Anchor offset calculation (grab_anchor=0.5 when clicking track)
         *    new_pos = (mouse_major - track_start) - (0.5 * THUMB_WIDTH)
         *            = (150 - 5) - 6
         *            = 139.0
         *
         * 5. Ratio calculation
         *    ratio = new_pos / track_length
         *          = 139 / 178 ≈ 0.7809
         *
         * 6. Value conversion (rounded to step=1.0)
         *    value = 0 + 0.7809 * (100 - 0) ≈ 78.09 → 78.0
         *****************************************************************/
        harness.mouse_move(Point::new(
            theme::SLIDER_DEFAULT_WIDTH * 0.75, // X coordinate: 200 * 0.75 = 150.0
            theme::SLIDER_DEFAULT_HEIGHT * 0.5, // Y coordinate remains centered
        ));

        // Verify value change event
        assert_eq!(
            harness.pop_action(),
            Some((Action::SliderValueChanged(78.0), slider_id)),
            "\nCalculation details:\n\
            Track length: 200 - 2*5 = 190\n\
            Movable range: 190 - 12 = 178\n\
            Mouse relative position: 150 - 5 = 145\n\
            Anchor offset: 145 - (12/2) = 139\n\
            Ratio: 139/178 ≈ 0.7809\n\
            Calculated value: 0.7809*100 = 78.09 → rounded to 78"
        );

        /*****************************************************************
         * Phase 3: Simulate repeat press (verify state changes)
         *****************************************************************/
        harness.mouse_button_press(PointerButton::Primary);
        assert_eq!(
            harness.pop_action(),
            Some((Action::SliderEditingChanged(true), slider_id))
        );

        /*****************************************************************
         * Phase 4: Release mouse (end editing)
         *****************************************************************/
        harness.mouse_button_release(PointerButton::Primary);
        assert_eq!(
            harness.pop_action(),
            Some((Action::SliderEditingChanged(false), slider_id))
        );
    }

    #[test]
    fn vertical_slider_interaction() {
        let [slider_id] = widget_ids();
        let slider = Slider::new(Axis::Vertical, 0.0, 100.0, 50.0).with_id(slider_id);

        let mut harness = TestHarness::create_with_size(slider, Size::new(40.0, 200.0));

        /*****************************************************************
         * Phase 1: Vertical slider drag calculation
         * Vertical coordinate system is reversed (top=max, bottom=min)
         * Test drag to height 60px (total height 200, track range 190)
         *
         * Calculation steps:
         * 1. Track dimensions
         *    - Total height: 200px
         *    - Track padding: 5px (TRACK_PADDING)
         *    - Thumb height: 12px (THUMB_WIDTH, swapped for vertical slider)
         *    - Track height: 200 - 2*5 = 190px
         *    - Track Y range: 5px to 195px
         *
         * 2. Movable range
         *    - track_length = track_height - thumb_height = 190 - 12 = 178px
         *
         * 3. Click position conversion
         *    - Mouse Y position: 60px (relative to window)
         *    - Relative to track start: 60 - 5 = 55px
         *
         * 4. Anchor offset correction (grab_anchor=0.5 when clicking track)
         *    - new_pos = 55px - (12px * 0.5) = 55 - 6 = 49px
         *
         * 5. Reversed ratio calculation (vertical axis)
         *    - ratio = 1.0 - (new_pos / track_length) = 1 - (49/178) ≈ 0.7247
         *
         * 6. Value calculation (rounded to 1 decimal place)
         *    - value = 0.7247 * 100 = 72.47 → rounded to 72.0
         *****************************************************************/
        harness.mouse_move_to(slider_id);
        harness.mouse_button_press(PointerButton::Primary);
        assert_eq!(
            harness.pop_action(),
            Some((Action::SliderEditingChanged(true), slider_id)),
            "Vertical slider should trigger editing on press"
        );

        // Move to Y=60px (X remains centered at 20px)
        harness.mouse_move(Point::new(20.0, 60.0));
        assert_eq!(
            harness.pop_action(),
            Some((Action::SliderValueChanged(72.0), slider_id)), // 72.47 rounded
            "Vertical drag to 60px should trigger value change to 72.0"
        );

        // Release mouse to end editing
        harness.mouse_button_release(PointerButton::Primary);
        assert_eq!(
            harness.pop_action(),
            Some((Action::SliderEditingChanged(false), slider_id)),
            "Should end editing on release"
        );

        // Verify final value
        let widget = harness.get_widget(slider_id).downcast::<Slider>().unwrap();
        assert_eq!(
            widget.value, 72.0,
            "Vertical slider final value should be 72.0"
        );
        assert!(harness.pop_action().is_none(), "No unhandled actions");
    }

    #[test]
    fn slider_min_max_value_interaction() {
        let [slider_id] = widget_ids();
        let slider = Slider::new(Axis::Horizontal, 0.0, 100.0, 0.0).with_id(slider_id);

        let mut harness = TestHarness::create_with_size(slider, TEST_SIZE);

        /*****************************************************************
         * Test dragging from minimum value to below minimum
         *****************************************************************/
        harness.mouse_move_to(slider_id);
        harness.mouse_button_press(PointerButton::Primary);
        assert_eq!(
            harness.pop_action(),
            Some((Action::SliderValueChanged(50.0), slider_id)),
            "Should remain at 0 when below minimum"
        );
        assert_eq!(
            harness.pop_action(),
            Some((Action::SliderEditingChanged(true), slider_id))
        );

        // Drag to -50px (will be clamped to 0)
        harness.mouse_move(Point::new(-50.0, 20.0));
        assert_eq!(
            harness.pop_action(),
            Some((Action::SliderValueChanged(0.0), slider_id)),
            "Should remain at 0 when below minimum"
        );

        harness.mouse_button_release(PointerButton::Primary);
        assert_eq!(
            harness.pop_action(),
            Some((Action::SliderEditingChanged(false), slider_id))
        );

        // Verify final value
        let widget = harness.get_widget(slider_id).downcast::<Slider>().unwrap();
        assert_eq!(widget.value, 0.0, "Value should remain at minimum 0");
    }

    #[test]
    fn slider_property_update() {
        let [slider_id] = widget_ids();
        let slider = Slider::new(Axis::Horizontal, 0.0, 100.0, 50.0).with_id(slider_id);

        let mut harness = TestHarness::create_with_size(slider, TEST_SIZE);

        // Modify properties and verify rendering updates
        harness.edit_widget(slider_id, |mut slider| {
            let mut slider = slider.downcast();
            Slider::set_thumb_color(&mut slider, Color::from_rgb8(0, 255, 0));
            Slider::set_track_color(&mut slider, Color::from_rgb8(255, 0, 0));
        });

        assert_render_snapshot!(harness, "slider_updated_colors");
    }
}
