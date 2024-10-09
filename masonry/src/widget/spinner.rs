// Copyright 2020 the Xilem Authors and the Druid Authors
// SPDX-License-Identifier: Apache-2.0

//! An animated spinner widget.

use std::f64::consts::PI;

use accesskit::{NodeBuilder, Role};
use smallvec::SmallVec;
use tracing::{trace_span, Span};
use vello::kurbo::{Affine, Cap, Line, Stroke};
use vello::Scene;

use crate::widget::WidgetMut;
use crate::{
    theme, AccessCtx, AccessEvent, BoxConstraints, Color, EventCtx, LayoutCtx, LifeCycle,
    LifeCycleCtx, PaintCtx, Point, PointerEvent, RegisterCtx, Size, StatusChange, TextEvent, Vec2,
    Widget, WidgetId,
};

// TODO - Set color
/// An animated spinner widget for showing a loading state.
///
/// To customize the spinner's size, you can place it inside a [`SizedBox`]
/// that has a fixed width and height.
///
/// [`SizedBox`]: crate::widget::SizedBox
pub struct Spinner {
    t: f64,
    color: Color,
}

// --- MARK: BUILDERS ---
impl Spinner {
    /// Create a spinner widget
    pub fn new() -> Spinner {
        Spinner::default()
    }

    /// Builder-style method for setting the spinner's color.
    pub fn with_color(mut self, color: impl Into<Color>) -> Self {
        self.color = color.into();
        self
    }
}

const DEFAULT_SPINNER_COLOR: Color = theme::TEXT_COLOR;

impl Default for Spinner {
    fn default() -> Self {
        Spinner {
            t: 0.0,
            color: DEFAULT_SPINNER_COLOR,
        }
    }
}

// --- MARK: WIDGETMUT ---
impl WidgetMut<'_, Spinner> {
    /// Set the spinner's color.
    pub fn set_color(&mut self, color: impl Into<Color>) {
        self.widget.color = color.into();
        self.ctx.request_paint();
    }

    /// Reset the spinner's color to its default value.
    pub fn reset_color(&mut self) {
        self.set_color(DEFAULT_SPINNER_COLOR);
    }
}

// --- MARK: IMPL WIDGET ---
impl Widget for Spinner {
    fn on_pointer_event(&mut self, _ctx: &mut EventCtx, _event: &PointerEvent) {}

    fn on_text_event(&mut self, _ctx: &mut EventCtx, _event: &TextEvent) {}

    fn on_access_event(&mut self, _ctx: &mut EventCtx, _event: &AccessEvent) {}

    fn register_children(&mut self, _ctx: &mut RegisterCtx) {}

    fn on_status_change(&mut self, _ctx: &mut LifeCycleCtx, _event: &StatusChange) {}

    fn lifecycle(&mut self, ctx: &mut LifeCycleCtx, event: &LifeCycle) {
        match event {
            LifeCycle::WidgetAdded => {
                ctx.request_anim_frame();
                ctx.request_paint();
            }
            LifeCycle::AnimFrame(interval) => {
                self.t += (*interval as f64) * 1e-9;
                if self.t >= 1.0 {
                    self.t = self.t.rem_euclid(1.0);
                }
                ctx.request_anim_frame();
                ctx.request_paint();
            }
            _ => (),
        }
    }

    fn layout(&mut self, _ctx: &mut LayoutCtx, bc: &BoxConstraints) -> Size {
        if bc.is_width_bounded() && bc.is_height_bounded() {
            bc.max()
        } else {
            bc.constrain(Size::new(
                theme::BASIC_WIDGET_HEIGHT,
                theme::BASIC_WIDGET_HEIGHT,
            ))
        }
    }

    fn paint(&mut self, ctx: &mut PaintCtx, scene: &mut Scene) {
        let t = self.t;
        let (width, height) = (ctx.size().width, ctx.size().height);
        let center = Point::new(width / 2.0, height / 2.0);
        let (r, g, b, original_alpha) = {
            let c = self.color;
            (c.r, c.g, c.b, c.a)
        };
        let scale_factor = width.min(height) / 40.0;

        for step in 1..=12 {
            let step = f64::from(step);
            let fade_t = (t * 12.0 + 1.0).trunc();
            let fade = ((fade_t + step).rem_euclid(12.0) / 12.0) + 1.0 / 12.0;
            let angle = Vec2::from_angle((step / 12.0) * -2.0 * PI);
            let ambit_start = center + (10.0 * scale_factor * angle);
            let ambit_end = center + (20.0 * scale_factor * angle);
            let alpha = (fade * original_alpha as f64) as u8;
            let color = Color::rgba8(r, g, b, alpha);

            scene.stroke(
                &Stroke::new(3.0 * scale_factor).with_caps(Cap::Square),
                Affine::IDENTITY,
                color,
                None,
                &Line::new(ambit_start, ambit_end),
            );
        }
    }

    fn accessibility_role(&self) -> Role {
        // Don't like to use that role, but I'm not seeing
        // anything that matches in accesskit::Role
        Role::Unknown
    }

    fn accessibility(&mut self, _ctx: &mut AccessCtx, _node: &mut NodeBuilder) {}

    fn children_ids(&self) -> SmallVec<[WidgetId; 16]> {
        SmallVec::new()
    }

    fn make_trace_span(&self) -> Span {
        trace_span!("Spinner")
    }
}

// --- MARK: TESTS ---
#[cfg(test)]
mod tests {
    use super::*;
    use crate::assert_render_snapshot;
    use crate::testing::TestHarness;

    #[test]
    fn simple_spinner() {
        let spinner = Spinner::new();

        let mut harness = TestHarness::create(spinner);
        assert_render_snapshot!(harness, "spinner_init");

        harness.animate_ms(700);
        assert_render_snapshot!(harness, "spinner_700ms");

        harness.animate_ms(400);
        assert_render_snapshot!(harness, "spinner_1100ms");
    }

    #[test]
    fn edit_spinner() {
        let image_1 = {
            let spinner = Spinner::new().with_color(Color::PURPLE);

            let mut harness = TestHarness::create_with_size(spinner, Size::new(30.0, 30.0));
            harness.render()
        };

        let image_2 = {
            let spinner = Spinner::new();

            let mut harness = TestHarness::create_with_size(spinner, Size::new(30.0, 30.0));

            harness.edit_root_widget(|mut spinner| {
                let mut spinner = spinner.downcast::<Spinner>();
                spinner.set_color(Color::PURPLE);
            });

            harness.render()
        };

        // We don't use assert_eq because we don't want rich assert
        assert!(image_1 == image_2);
    }
}
