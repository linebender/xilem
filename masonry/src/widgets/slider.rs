// Copyright 2025 the Xilem Authors
// SPDX-License-Identifier: Apache-2.0

use tracing::{Span, trace_span};
use ui_events::pointer::PointerButton;
use vello::Scene;
use vello::kurbo::{Affine, Circle, Point, Rect, Size, Stroke};
use vello::peniko::{Brush, Color, Fill};

use crate::accesskit::{Node, Role};
use crate::core::keyboard::{Key, NamedKey};
use crate::core::{
    AccessCtx, AccessEvent, BoxConstraints, ChildrenIds, EventCtx, LayoutCtx, PaintCtx,
    PointerEvent, PropertiesMut, PropertiesRef, RegisterCtx, TextEvent, Update, UpdateCtx, Widget,
    WidgetId, WidgetMut,
};
use crate::theme;

/// A widget that allows a user to select a value from a continuous range.
pub struct Slider {
    // --- Logic ---
    min: f64,
    max: f64,
    value: f64,
    step: Option<f64>,
    // --- State ---
    is_focused: bool,
    disabled: bool,
    // --- Style ---
    track_color: Option<Color>,
    active_track_color: Option<Color>,
    track_thickness: Option<f64>,
    thumb_color: Option<Color>,
    thumb_radius: Option<f64>,
}

impl Slider {
    /// Creates a new `Slider`.
    pub fn new(min: f64, max: f64, value: f64) -> Self {
        Self {
            min,
            max,
            value: value.clamp(min, max),
            step: None,
            is_focused: false,
            disabled: false,
            track_color: None,
            active_track_color: None,
            track_thickness: None,
            thumb_color: None,
            thumb_radius: None,
        }
    }

    /// Configures the stepping interval of the slider.
    pub fn with_step(mut self, step: f64) -> Self {
        self.set_step_internal(Some(step));
        self
    }

    /// Configures the disabled state of the slider.
    pub fn with_disabled(mut self, disabled: bool) -> Self {
        self.disabled = disabled;
        self
    }

    /// Configures the color of the inactive part of the track.
    pub fn with_track_color(mut self, color: Color) -> Self {
        self.track_color = Some(color);
        self
    }

    /// Configures the color of the active part of the track and the thumb border.
    pub fn with_active_track_color(mut self, color: Color) -> Self {
        self.active_track_color = Some(color);
        self
    }

    /// Configures the thickness (height) of the track.
    pub fn with_track_thickness(mut self, thickness: f64) -> Self {
        self.track_thickness = Some(thickness);
        self
    }

    /// Configures the main fill color of the thumb.
    pub fn with_thumb_color(mut self, color: Color) -> Self {
        self.thumb_color = Some(color);
        self
    }

    /// Configures the base radius of the thumb.
    pub fn with_thumb_radius(mut self, radius: f64) -> Self {
        self.thumb_radius = Some(radius);
        self
    }

    // --- Upd methods from `rebuild` ---

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

    /// Sets the disabled state of the slider.
    pub fn set_disabled(this: &mut WidgetMut<'_, Self>, disabled: bool) {
        if this.widget.disabled != disabled {
            this.widget.disabled = disabled;
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

    /// sets track color
    pub fn set_track_color(this: &mut WidgetMut<'_, Self>, color: Option<Color>) {
        if this.widget.track_color != color {
            this.widget.track_color = color;
            this.ctx.request_render();
        }
    }

    /// sets active track color
    pub fn set_active_track_color(this: &mut WidgetMut<'_, Self>, color: Option<Color>) {
        if this.widget.active_track_color != color {
            this.widget.active_track_color = color;
            this.ctx.request_render();
        }
    }

    /// sets track thiknes
    pub fn set_track_thickness(this: &mut WidgetMut<'_, Self>, thickness: Option<f64>) {
        if this.widget.track_thickness != thickness {
            this.widget.track_thickness = thickness;
            this.ctx.request_layout();
        }
    }

    /// sets thumb color
    pub fn set_thumb_color(this: &mut WidgetMut<'_, Self>, color: Option<Color>) {
        if this.widget.thumb_color != color {
            this.widget.thumb_color = color;
            this.ctx.request_render();
        }
    }
    /// sets thumb radius
    pub fn set_thumb_radius(this: &mut WidgetMut<'_, Self>, radius: Option<f64>) {
        if this.widget.thumb_radius != radius {
            this.widget.thumb_radius = radius;
            this.ctx.request_layout();
        }
    }

    // --- Logic ---

    fn set_step_internal(&mut self, step: Option<f64>) {
        self.step = step.filter(|s| *s > 0.0);
        let clamped_value = self.value.clamp(self.min, self.max);
        self.value = if let Some(s) = self.step {
            ((clamped_value / s).round() * s).clamp(self.min, self.max)
        } else {
            clamped_value
        };
    }

    fn update_value_from_position(&mut self, x: f64, width: f64) -> bool {
        let base_thumb_radius = self.thumb_radius.unwrap_or(6.0);
        let thumb_radius = if self.is_focused {
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

impl Widget for Slider {
    type Action = f64;

    fn accepts_pointer_interaction(&self) -> bool {
        !self.disabled
    }

    fn accepts_focus(&self) -> bool {
        !self.disabled
    }

    fn on_pointer_event(
        &mut self,
        ctx: &mut EventCtx<'_>,
        _props: &mut PropertiesMut<'_>,
        event: &PointerEvent,
    ) {
        if self.disabled {
            return;
        }
        match event {
            PointerEvent::Down {
                button: Some(PointerButton::Primary),
                state,
                ..
            } => {
                ctx.request_focus();
                ctx.capture_pointer();
                let local_pos = ctx.local_position(state.position);
                if self.update_value_from_position(local_pos.x, ctx.size().width) {
                    ctx.submit_action::<f64>(self.value);
                }
            }
            PointerEvent::Move(e) => {
                if ctx.is_active() {
                    let local_pos = ctx.local_position(e.current.position);
                    if self.update_value_from_position(local_pos.x, ctx.size().width) {
                        ctx.submit_action::<f64>(self.value);
                    }
                    ctx.request_render();
                }
            }
            PointerEvent::Up {
                button: Some(PointerButton::Primary),
                ..
            } => {
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
        if self.disabled || !self.is_focused {
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
            Update::FocusChanged(focused) => {
                self.is_focused = *focused;
                ctx.request_render();
            }
            Update::HoveredChanged(_) | Update::ActiveChanged(_) => {
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
        if self.disabled {
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
            accesskit::Action::SetValue => {
                // Dont know how use and change this value...
            }
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

    fn layout(
        &mut self,
        _ctx: &mut LayoutCtx<'_>,
        _props: &mut PropertiesMut<'_>,
        bc: &BoxConstraints,
    ) -> Size {
        let base_thumb_radius = self.thumb_radius.unwrap_or(6.0);
        let height = (base_thumb_radius * 2.0).max(self.track_thickness.unwrap_or(4.0)) + 16.0;
        let width = bc.max().width.clamp(100.0, 200.0);
        Size::new(width, height)
    }

    fn paint(&mut self, ctx: &mut PaintCtx<'_>, _props: &PropertiesRef<'_>, scene: &mut Scene) {
        // --- 1. Get parameters and resolve colors ---
        let track_color = self.track_color.unwrap_or(theme::ZYNC_800);
        let active_track_color = self.active_track_color.unwrap_or(theme::ACCENT_COLOR);
        let thumb_color = self.thumb_color.unwrap_or(theme::TEXT_COLOR);
        let track_thickness = self.track_thickness.unwrap_or(4.0);
        let base_thumb_radius = self.thumb_radius.unwrap_or(6.0);
        let thumb_border_width = 2.0;

        let disabled_alpha = 0.4;
        let final_track_color = if self.disabled {
            track_color.with_alpha(disabled_alpha)
        } else {
            track_color
        };
        let final_active_track_color = if self.disabled {
            active_track_color.with_alpha(disabled_alpha)
        } else {
            active_track_color
        };
        let final_thumb_color = if self.disabled {
            thumb_color.with_alpha(disabled_alpha)
        } else {
            thumb_color
        };

        // --- 2. Calculate geometry based on state ---
        let size = ctx.size();
        let thumb_radius = if self.disabled {
            base_thumb_radius
        } else if ctx.is_active() {
            base_thumb_radius + 2.0
        } else if ctx.is_hovered() || self.is_focused {
            base_thumb_radius + 1.0
        } else {
            base_thumb_radius
        };
        let track_start_x = thumb_radius;
        let track_width = (size.width - thumb_radius * 2.0).max(0.0);
        let track_y = (size.height - track_thickness) / 2.0;

        // --- 3. Paint inactive track ---
        let track_rect = Rect::new(
            track_start_x,
            track_y,
            track_start_x + track_width,
            track_y + track_thickness,
        );
        scene.fill(
            Fill::NonZero,
            Affine::IDENTITY,
            &Brush::Solid(final_track_color),
            None,
            &track_rect.to_rounded_rect(track_thickness / 2.0),
        );

        // --- 4. Paint active track ---
        let progress = (self.value - self.min) / (self.max - self.min).max(f64::EPSILON);
        let active_track_width = progress * track_width;
        if active_track_width > 0.0 {
            let active_track_rect = Rect::new(
                track_start_x,
                track_y,
                track_start_x + active_track_width,
                track_y + track_thickness,
            );
            scene.fill(
                Fill::NonZero,
                Affine::IDENTITY,
                &Brush::Solid(final_active_track_color),
                None,
                &active_track_rect.to_rounded_rect(track_thickness / 2.0),
            );
        }

        // --- 5. Paint thumb ---
        let thumb_x = track_start_x + active_track_width;
        let thumb_y = size.height / 2.0;
        let thumb_circle = Circle::new(Point::new(thumb_x, thumb_y), thumb_radius);

        scene.fill(
            Fill::NonZero,
            Affine::IDENTITY,
            &Brush::Solid(final_thumb_color),
            None,
            &thumb_circle,
        );
        scene.stroke(
            &Stroke::new(thumb_border_width),
            Affine::IDENTITY,
            &Brush::Solid(final_active_track_color),
            None,
            &thumb_circle,
        );

        // --- 6. Paint focus ring ---
        if self.is_focused && !self.disabled {
            let focus_rect = ctx.size().to_rect().inset(2.0);
            let focus_color =
                theme::FOCUS_COLOR.with_alpha(if ctx.is_active() { 1.0 } else { 0.5 } as f32);
            scene.stroke(
                &Stroke::new(1.0),
                Affine::IDENTITY,
                &Brush::Solid(focus_color),
                None,
                &focus_rect.to_rounded_rect(4.0),
            );
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

// --- MARK: TESTS ---
#[cfg(test)]
mod tests {
    use masonry_core::core::NewWidget;
    use vello::kurbo::{Point, Size};

    use super::*;
    use crate::core::{PointerButton, TextEvent};
    use crate::testing::{TestHarness, assert_render_snapshot};
    use crate::theme::default_property_set;

    #[test]
    fn slider_initial_state() {
        let widget = NewWidget::new(Slider::new(0.0, 100.0, 25.0));
        let mut harness =
            TestHarness::create_with_size(default_property_set(), widget, Size::new(200.0, 32.0));

        assert_render_snapshot!(harness, "slider_initial_state");
    }

    #[test]
    fn slider_drag_interaction() {
        let widget = NewWidget::new(Slider::new(0.0, 100.0, 25.0));
        let mut harness =
            TestHarness::create_with_size(default_property_set(), widget, Size::new(200.0, 32.0));
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
        let widget = NewWidget::new(Slider::new(0.0, 100.0, 50.0).with_step(10.0));
        let mut harness =
            TestHarness::create_with_size(default_property_set(), widget, Size::new(200.0, 32.0));
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
        let widget = NewWidget::new(Slider::new(0.0, 100.0, 50.0).with_disabled(true));
        let mut harness =
            TestHarness::create_with_size(default_property_set(), widget, Size::new(200.0, 32.0));

        assert_render_snapshot!(harness, "slider_disabled");
        assert!(harness.pop_action::<f64>().is_none());
    }
}
