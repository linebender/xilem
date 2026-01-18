// Copyright 2025 the Xilem Authors
// SPDX-License-Identifier: Apache-2.0

//! A slider widget.

use std::any::TypeId;

use accesskit::{ActionData, Node, Role};
use tracing::{Span, trace_span};
use vello::Scene;

use crate::core::keyboard::{Key, NamedKey};
use crate::core::pointer::PointerButton;
use crate::core::{
    AccessCtx, AccessEvent, ChildrenIds, EventCtx, HasProperty, LayoutCtx, MeasureCtx, PaintCtx,
    PointerButtonEvent, PointerEvent, PointerUpdate, PropertiesMut, PropertiesRef, RegisterCtx,
    TextEvent, Update, UpdateCtx, Widget, WidgetId, WidgetMut,
};
use crate::kurbo::{Axis, Circle, Point, Rect, Size};
use crate::layout::LenReq;
use crate::peniko::Fill;
use crate::properties::{Background, BarColor, ThumbColor, ThumbRadius, TrackThickness};
use crate::theme;
use crate::util::{fill, include_screenshot, stroke};

/// A widget that allows a user to select a value from a continuous range.
///
#[doc = include_screenshot!("slider_initial_state.png", "Slider.")]
pub struct Slider {
    // --- Logic ---
    min: f64,
    max: f64,
    value: f64,
    step: Option<f64>,
}

// --- MARK: BUILDERS
impl Slider {
    /// Creates a new `Slider`.
    pub fn new(min: f64, max: f64, value: f64) -> Self {
        Self {
            min,
            max,
            value: value.clamp(min, max),
            step: None,
        }
    }

    /// Configures the stepping interval of the slider.
    pub fn with_step(mut self, step: f64) -> Self {
        self.set_step_internal(Some(step));
        self
    }
}

// --- MARK: METHODS
impl Slider {
    fn set_step_internal(&mut self, step: Option<f64>) {
        self.step = step.filter(|s| *s > 0.0);
        let clamped_value = self.value.clamp(self.min, self.max);
        self.value = if let Some(s) = self.step {
            ((clamped_value / s).round() * s).clamp(self.min, self.max)
        } else {
            clamped_value
        };
    }

    fn update_value_from_position(
        &mut self,
        x: f64,
        width: f64,
        ThumbRadius(base_thumb_radius): ThumbRadius,
        is_focused: bool,
    ) -> bool {
        let thumb_radius = if is_focused {
            base_thumb_radius + 2.0
        } else {
            base_thumb_radius
        };
        let track_start_x = thumb_radius;
        let track_width = (width - thumb_radius * 2.0).max(0.0);
        let relative_x = x - track_start_x;
        let progress = (relative_x / track_width).clamp(0.0, 1.0);
        let new_value = self.min + progress * (self.max - self.min);
        let old_value = self.value;
        let final_value = if let Some(step) = self.step {
            ((new_value / step).round() * step).clamp(self.min, self.max)
        } else {
            new_value.clamp(self.min, self.max)
        };
        if (final_value - old_value).abs() > f64::EPSILON {
            self.value = final_value;
            true
        } else {
            false
        }
    }
}

// --- MARK: WIDGETMUT
impl Slider {
    /// Sets the current value of the slider.
    pub fn set_value(this: &mut WidgetMut<'_, Self>, value: f64) {
        let clamped_value = value.clamp(this.widget.min, this.widget.max);
        let new_value = if let Some(step) = this.widget.step {
            ((clamped_value / step).round() * step).clamp(this.widget.min, this.widget.max)
        } else {
            clamped_value
        };
        if (new_value - this.widget.value).abs() > f64::EPSILON {
            this.widget.value = new_value;
            this.ctx.request_render();
        }
    }

    /// Sets or removes the stepping interval of the slider.
    pub fn set_step(this: &mut WidgetMut<'_, Self>, step: Option<f64>) {
        let filtered_step = step.filter(|s| *s > 0.0);
        if this.widget.step != filtered_step {
            this.widget.set_step_internal(filtered_step);
            this.ctx.request_render();
        }
    }

    /// Sets the range (min and max) of the slider.
    pub fn set_range(this: &mut WidgetMut<'_, Self>, min: f64, max: f64) {
        if this.widget.min != min || this.widget.max != max {
            this.widget.min = min;
            this.widget.max = max;
            Self::set_value(this, this.widget.value);
        }
    }
}

impl HasProperty<Background> for Slider {}
impl HasProperty<BarColor> for Slider {}
impl HasProperty<TrackThickness> for Slider {}
impl HasProperty<ThumbColor> for Slider {}
impl HasProperty<ThumbRadius> for Slider {}

// --- MARK: IMPL WIDGET
impl Widget for Slider {
    type Action = f64;

    fn accepts_focus(&self) -> bool {
        true
    }

    fn on_pointer_event(
        &mut self,
        ctx: &mut EventCtx<'_>,
        props: &mut PropertiesMut<'_>,
        event: &PointerEvent,
    ) {
        if ctx.is_disabled() {
            return;
        }
        match event {
            PointerEvent::Down(PointerButtonEvent {
                button: Some(PointerButton::Primary),
                state,
                ..
            }) => {
                ctx.request_focus();
                ctx.capture_pointer();
                let local_pos = ctx.local_position(state.position);
                if self.update_value_from_position(
                    local_pos.x,
                    ctx.size().width,
                    *props.get(),
                    ctx.is_focus_target(),
                ) {
                    ctx.submit_action::<f64>(self.value);
                }
            }
            PointerEvent::Move(PointerUpdate { current, .. }) => {
                if ctx.is_active() {
                    let local_pos = ctx.local_position(current.position);
                    if self.update_value_from_position(
                        local_pos.x,
                        ctx.size().width,
                        *props.get(),
                        ctx.is_focus_target(),
                    ) {
                        ctx.submit_action::<f64>(self.value);
                    }
                    ctx.request_render();
                }
            }
            PointerEvent::Up(PointerButtonEvent {
                button: Some(PointerButton::Primary),
                ..
            }) => {
                if ctx.is_active() {
                    ctx.release_pointer();
                }
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
        if ctx.is_disabled() || !ctx.is_focus_target() {
            return;
        }

        if let TextEvent::Keyboard(key_event) = event {
            if key_event.state.is_up() {
                return;
            }

            let mut new_value = self.value;
            let step = self
                .step
                .unwrap_or((self.max - self.min) / 100.0)
                .max(f64::EPSILON);
            let big_step = step * 10.0;

            match &key_event.key {
                Key::Named(NamedKey::ArrowLeft) | Key::Named(NamedKey::ArrowDown) => {
                    new_value -= if key_event.modifiers.shift() {
                        big_step
                    } else {
                        step
                    }
                }
                Key::Named(NamedKey::ArrowRight) | Key::Named(NamedKey::ArrowUp) => {
                    new_value += if key_event.modifiers.shift() {
                        big_step
                    } else {
                        step
                    }
                }
                Key::Named(NamedKey::Home) => new_value = self.min,
                Key::Named(NamedKey::End) => new_value = self.max,
                _ => return,
            }

            if new_value != self.value {
                let clamped_value = new_value.clamp(self.min, self.max);
                let final_value = if let Some(s) = self.step {
                    ((clamped_value / s).round() * s).clamp(self.min, self.max)
                } else {
                    clamped_value
                };

                if (final_value - self.value).abs() > f64::EPSILON {
                    self.value = final_value;
                    ctx.request_render();
                    ctx.submit_action::<f64>(self.value);
                }
            }
        }
    }

    fn update(&mut self, ctx: &mut UpdateCtx<'_>, _props: &mut PropertiesMut<'_>, event: &Update) {
        match event {
            Update::FocusChanged(_) | Update::HoveredChanged(_) | Update::ActiveChanged(_) => {
                ctx.request_render();
            }
            _ => {}
        }
    }

    fn on_access_event(
        &mut self,
        ctx: &mut EventCtx<'_>,
        _props: &mut PropertiesMut<'_>,
        event: &AccessEvent,
    ) {
        if ctx.is_disabled() {
            return;
        }

        let step = self
            .step
            .unwrap_or((self.max - self.min) / 100.0)
            .max(f64::EPSILON);
        let mut new_value = self.value;

        match event.action {
            accesskit::Action::Increment => {
                new_value += step;
            }
            accesskit::Action::Decrement => {
                new_value -= step;
            }
            accesskit::Action::SetValue => match &event.data {
                Some(ActionData::NumericValue(value)) => new_value = *value,
                Some(ActionData::Value(value)) => {
                    if let Ok(value) = value.parse() {
                        new_value = value;
                    }
                }
                _ => {}
            },
            _ => return,
        }

        if (new_value - self.value).abs() > f64::EPSILON {
            let clamped_value = new_value.clamp(self.min, self.max);
            self.value = if let Some(s) = self.step {
                ((clamped_value / s).round() * s).clamp(self.min, self.max)
            } else {
                clamped_value
            };
            ctx.request_render();
            ctx.submit_action::<f64>(self.value);
        }
    }

    fn register_children(&mut self, _ctx: &mut RegisterCtx<'_>) {}

    fn property_changed(&mut self, ctx: &mut UpdateCtx<'_>, property_type: TypeId) {
        Background::prop_changed(ctx, property_type);
        BarColor::prop_changed(ctx, property_type);
        TrackThickness::prop_changed(ctx, property_type);
        ThumbColor::prop_changed(ctx, property_type);
        ThumbRadius::prop_changed(ctx, property_type);
    }

    fn measure(
        &mut self,
        _ctx: &mut MeasureCtx<'_>,
        props: &PropertiesRef<'_>,
        axis: Axis,
        len_req: LenReq,
        _cross_length: Option<f64>,
    ) -> f64 {
        // TODO: Remove HACK: Until scale factor rework happens, just pretend it's always 1.0.
        //       https://github.com/linebender/xilem/issues/1264
        let scale = 1.0;

        match axis {
            Axis::Horizontal => match len_req {
                // TODO: Move this 100. to theme?
                LenReq::MinContent | LenReq::MaxContent => 100. * scale,
                LenReq::FitContent(space) => space,
            },
            Axis::Vertical => {
                let thumb_radius = props.get::<ThumbRadius>();
                let track_thickness = props.get::<TrackThickness>();

                let thumb_length = thumb_radius.0 * 2.0 * scale;
                let track_length = track_thickness.0 * scale;
                // TODO: Move the padding 16. to theme or make it otherwise configurable?
                let padding_length = 16. * scale;

                thumb_length.max(track_length) + padding_length
            }
        }
    }

    fn layout(&mut self, _ctx: &mut LayoutCtx<'_>, _props: &PropertiesRef<'_>, _size: Size) {}

    fn paint(&mut self, ctx: &mut PaintCtx<'_>, props: &PropertiesRef<'_>, scene: &mut Scene) {
        // Get parameters and resolve colors
        let track_color = if props.contains::<Background>() {
            props.get::<Background>()
        } else {
            &Background::Color(theme::ZYNC_800)
        };
        let active_track_color = if props.contains::<BarColor>() {
            props.get::<BarColor>().0
        } else {
            theme::ACCENT_COLOR
        };
        let thumb_color = props.get::<ThumbColor>().0;
        let track_thickness = props.get::<TrackThickness>().0;
        let base_thumb_radius = props.get::<ThumbRadius>().0;
        let thumb_border_width = 2.0;

        // Calculate geometry based on state
        let size = ctx.size();
        let thumb_radius = if ctx.is_active() {
            base_thumb_radius + 2.0
        } else if ctx.is_hovered() || ctx.is_focus_target() {
            base_thumb_radius + 1.0
        } else {
            base_thumb_radius
        };
        let track_start_x = thumb_radius;
        let track_width = (size.width - thumb_radius * 2.0).max(0.0);
        let track_y = (size.height - track_thickness) / 2.0;

        // Push semitransparent layer if disabled
        if ctx.is_disabled() {
            const DISABLED_ALPHA: f32 = 0.4;
            scene.push_layer(
                Fill::NonZero,
                crate::peniko::Mix::Normal,
                DISABLED_ALPHA,
                crate::kurbo::Affine::IDENTITY,
                &ctx.size().to_rect(),
            );
        }

        // Paint inactive track
        let track_rect = Rect::new(
            track_start_x,
            track_y,
            track_start_x + track_width,
            track_y + track_thickness,
        );
        fill(
            scene,
            &track_rect.to_rounded_rect(track_thickness / 2.0),
            &track_color.get_peniko_brush_for_rect(track_rect),
        );

        // Paint active track
        let progress = (self.value - self.min) / (self.max - self.min).max(f64::EPSILON);
        let active_track_width = progress * track_width;
        if active_track_width > 0.0 {
            let active_track_rect = Rect::new(
                track_start_x,
                track_y,
                track_start_x + active_track_width,
                track_y + track_thickness,
            );
            fill(
                scene,
                &active_track_rect.to_rounded_rect(track_thickness / 2.0),
                active_track_color,
            );
        }

        // Paint thumb
        let thumb_x = track_start_x + active_track_width;
        let thumb_y = size.height / 2.0;
        let thumb_circle = Circle::new(Point::new(thumb_x, thumb_y), thumb_radius);

        fill(scene, &thumb_circle, thumb_color);
        stroke(scene, &thumb_circle, active_track_color, thumb_border_width);

        // Paint focus ring
        if ctx.is_focus_target() && !ctx.is_disabled() {
            let focus_rect = ctx.size().to_rect().inset(2.0);
            let focus_color =
                theme::FOCUS_COLOR.with_alpha(if ctx.is_active() { 1.0 } else { 0.5 });
            stroke(scene, &focus_rect.to_rounded_rect(4.0), focus_color, 1.0);
        }

        // Pop the semitransparent layer
        if ctx.is_disabled() {
            scene.pop_layer();
        }
    }

    fn accessibility_role(&self) -> Role {
        Role::Slider
    }

    fn accessibility(
        &mut self,
        _ctx: &mut AccessCtx<'_>,
        _props: &PropertiesRef<'_>,
        node: &mut Node,
    ) {
        node.set_value(self.value.to_string());
        node.set_min_numeric_value(self.min);
        node.set_max_numeric_value(self.max);
        if let Some(step) = self.step {
            node.set_numeric_value_step(step);
        }
        node.add_action(accesskit::Action::SetValue);
        node.add_action(accesskit::Action::Increment);
        node.add_action(accesskit::Action::Decrement);
    }

    fn children_ids(&self) -> ChildrenIds {
        ChildrenIds::new()
    }

    fn make_trace_span(&self, id: WidgetId) -> Span {
        trace_span!("Slider", id = id.trace())
    }
}

// --- MARK: TESTS
#[cfg(test)]
mod tests {
    use super::*;
    use crate::core::{PointerButton, TextEvent};
    use crate::kurbo::{Point, Size};
    use crate::testing::{TestHarness, assert_render_snapshot};
    use crate::theme::test_property_set;

    #[test]
    fn slider_initial_state() {
        let widget = Slider::new(0.0, 100.0, 25.0).with_auto_id();
        let mut harness =
            TestHarness::create_with_size(test_property_set(), widget, Size::new(200.0, 32.0));

        assert_render_snapshot!(harness, "slider_initial_state");
    }

    #[test]
    fn slider_drag_interaction() {
        let widget = Slider::new(0.0, 100.0, 25.0).with_auto_id();
        let mut harness =
            TestHarness::create_with_size(test_property_set(), widget, Size::new(200.0, 32.0));
        let slider_id = harness.root_id();

        assert_render_snapshot!(harness, "slider_drag_initial_at_25");

        // 1. Move the mouse to the thumb position (25%) BEFORE clicking.
        harness.mouse_move(Point::new(53.0, 16.0));

        // 2. Press the mouse button.
        // This should not emit an action because the value does not change.
        harness.mouse_button_press(PointerButton::Primary);
        assert!(harness.pop_action::<f64>().is_none());

        // 3. Move to the new position (75%).
        // PosX for 75.0 = 8.0 + (184.0 * 0.75) = 146.0
        harness.mouse_move(Point::new(146.0, 16.0));

        assert_eq!(harness.pop_action::<f64>(), Some((75.0, slider_id)));
        assert_render_snapshot!(harness, "slider_drag_to_75");

        // Release the mouse
        harness.mouse_button_release(PointerButton::Primary);
        assert_render_snapshot!(harness, "slider_drag_released_at_75");
    }

    #[test]
    fn slider_keyboard_interaction() {
        let widget = Slider::new(0.0, 100.0, 50.0).with_step(10.0).with_auto_id();
        let mut harness =
            TestHarness::create_with_size(test_property_set(), widget, Size::new(200.0, 32.0));
        let slider_id = harness.root_id();

        harness.focus_on(Some(slider_id));
        assert_render_snapshot!(harness, "slider_keyboard_focused");

        harness.process_text_event(TextEvent::key_down(Key::Named(NamedKey::ArrowRight)));
        harness.process_text_event(TextEvent::key_up(Key::Named(NamedKey::ArrowRight)));

        assert_eq!(harness.pop_action::<f64>(), Some((60.0, slider_id)));
        assert_render_snapshot!(harness, "slider_keyboard_moved");
    }

    #[test]
    fn slider_disabled_state() {
        let mut widget = Slider::new(0.0, 100.0, 50.0).with_auto_id();
        widget.options.disabled = true;
        let mut harness =
            TestHarness::create_with_size(test_property_set(), widget, Size::new(200.0, 32.0));

        assert_render_snapshot!(harness, "slider_disabled");
        assert!(harness.pop_action::<f64>().is_none());
    }
}
