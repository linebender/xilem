// Copyright 2024 Retype15 (https://github.com/Retype15)
// SPDX-License-Identifier: Apache-2.0

use tracing::{Span, trace_span};
use vello::Scene;
use vello::kurbo::{Affine, Circle, Point, Rect, Size, Stroke};
use vello::peniko::{Brush, Color, Fill};

use crate::accesskit::{Node, Role};
use crate::core::{
    AccessCtx, AccessEvent, BoxConstraints, ChildrenIds, EventCtx, LayoutCtx, PaintCtx,
    PointerEvent, PropertiesMut, PropertiesRef, RegisterCtx, TextEvent, Update, UpdateCtx, Widget,
    WidgetId, WidgetMut,
};
use crate::theme;

/// A widget that allows a user to select a value from a continuous range.
pub struct Slider {
    min: f64,
    max: f64,
    value: f64,
    step: Option<f64>,
    is_dragging: bool,
    disabled: bool,
    track_color: Option<Color>,
    active_track_color: Option<Color>,
    thumb_color: Option<Color>,
    track_thickness: Option<f64>,
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
            is_dragging: false,
            disabled: false,
            track_color: None,
            active_track_color: None,
            thumb_color: None,
            track_thickness: None,
            thumb_radius: None,
        }
    }

    /// Sets the stepping interval of the slider.
    pub fn with_step(mut self, step: f64) -> Self {
        self.set_step_internal(Some(step));
        self
    }
    /// Sets the color of the inactive part of the track.
    pub fn with_track_color(mut self, color: Color) -> Self {
        self.track_color = Some(color);
        self
    }
    /// Sets the color of the active part of the track and the thumb border.
    pub fn with_active_track_color(mut self, color: Color) -> Self {
        self.active_track_color = Some(color);
        self
    }
    /// Sets the main fill color of the thumb.
    pub fn with_thumb_color(mut self, color: Color) -> Self {
        self.thumb_color = Some(color);
        self
    }
    /// Sets the thickness (height) of the track.
    pub fn with_track_thickness(mut self, thickness: f64) -> Self {
        self.track_thickness = Some(thickness);
        self
    }
    /// Sets the base radius of the thumb.
    pub fn with_thumb_radius(mut self, radius: f64) -> Self {
        self.thumb_radius = Some(radius);
        self
    }
    /// Sets the initial disabled state of the slider.
    pub fn with_disabled(mut self, disabled: bool) -> Self {
        self.disabled = disabled;
        self
    }

    /// Sets the current value of the slider.
    /// The value will be clamped to the slider's range and rounded to the nearest step.
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

    fn set_step_internal(&mut self, step: Option<f64>) {
        self.step = step.filter(|s| *s > 0.0);
        let clamped_value = self.value.clamp(self.min, self.max);
        self.value = if let Some(s) = self.step {
            ((clamped_value / s).round() * s).clamp(self.min, self.max)
        } else {
            clamped_value
        };
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

    /// Sets the disabled state of the slider.
    pub fn set_disabled(this: &mut WidgetMut<'_, Self>, disabled: bool) {
        if this.widget.disabled != disabled {
            this.widget.disabled = disabled;
            this.ctx.request_render();
        }
    }

    // --- Lógica Interna ---
    fn update_value_from_position(&mut self, x: f64, width: f64) -> bool {
        let base_thumb_radius = self.thumb_radius.unwrap_or(6.0);
        let thumb_radius = if self.is_dragging {
            base_thumb_radius + 2.0
        } else {
            base_thumb_radius
        };
        let track_start_x = thumb_radius;
        let track_width = width - (thumb_radius * 2.0);
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
            PointerEvent::Down { state, .. } => {
                self.is_dragging = true;
                ctx.request_render();
                ctx.capture_pointer();
                let local_pos = ctx.local_position(state.position);
                if self.update_value_from_position(local_pos.x, ctx.size().width) {
                    ctx.submit_action::<f64>(self.value);
                }
            }
            PointerEvent::Move(e) => {
                if self.is_dragging {
                    let local_pos = ctx.local_position(e.current.position);
                    if self.update_value_from_position(local_pos.x, ctx.size().width) {
                        ctx.submit_action::<f64>(self.value);
                    }
                    ctx.request_render();
                }
            }
            PointerEvent::Up { .. } => {
                if self.is_dragging {
                    self.is_dragging = false;
                    ctx.release_pointer();
                    ctx.request_render();
                }
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
        let base_thumb_radius = self.thumb_radius.unwrap_or(6.0);
        let height = (base_thumb_radius * 2.0).max(self.track_thickness.unwrap_or(4.0)) + 16.0;
        let width = bc.max().width.clamp(100.0, 200.0);
        Size::new(width, height)
    }

    fn paint(&mut self, ctx: &mut PaintCtx<'_>, _props: &PropertiesRef<'_>, scene: &mut Scene) {
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

        let size = ctx.size();
        let thumb_radius = if self.is_dragging {
            base_thumb_radius + 2.0
        } else {
            base_thumb_radius
        };
        let track_start_x = thumb_radius;
        let track_width = size.width - (thumb_radius * 2.0);

        let track_y = (size.height - track_thickness) / 2.0;
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
    }

    fn accessibility_role(&self) -> Role {
        Role::Slider
    }

    fn accessibility(
        &mut self,
        _ctx: &mut AccessCtx<'_>,
        _props: &PropertiesRef<'_>,
        _node: &mut Node,
    ) {
    }

    fn children_ids(&self) -> ChildrenIds {
        ChildrenIds::new()
    }

    fn make_trace_span(&self, id: WidgetId) -> Span {
        trace_span!("Slider", id = id.trace())
    }
}
