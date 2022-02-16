// Copyright 2020 The Druid Authors.

//! An animated spinner widget.

use smallvec::SmallVec;
use std::f64::consts::PI;
use tracing::trace;

use crate::widget::widget_view::WidgetRef;
use druid::kurbo::Line;
use druid::widget::prelude::*;
use druid::{theme, Color, KeyOrValue, Point, Vec2};

// TODO - Set color
/// An animated spinner widget for showing a loading state.
///
/// To customize the spinner's size, you can place it inside a [`SizedBox`]
/// that has a fixed width and height.
///
/// [`SizedBox`]: struct.SizedBox.html
pub struct Spinner {
    t: f64,
    color: KeyOrValue<Color>,
}

impl Spinner {
    /// Create a spinner widget
    pub fn new() -> Spinner {
        Spinner::default()
    }

    /// Builder-style method for setting the spinner's color.
    ///
    /// The argument can be either a `Color` or a [`Key<Color>`].
    ///
    /// [`Key<Color>`]: ../struct.Key.html
    pub fn with_color(mut self, color: impl Into<KeyOrValue<Color>>) -> Self {
        self.color = color.into();
        self
    }

    /// Set the spinner's color.
    ///
    /// The argument can be either a `Color` or a [`Key<Color>`].
    ///
    /// [`Key<Color>`]: ../struct.Key.html
    pub fn set_color(&mut self, color: impl Into<KeyOrValue<Color>>) {
        self.color = color.into();
    }
}

impl Default for Spinner {
    fn default() -> Self {
        Spinner {
            t: 0.0,
            color: theme::TEXT_COLOR.into(),
        }
    }
}

impl Widget for Spinner {
    fn on_event(&mut self, ctx: &mut EventCtx, event: &Event, _env: &Env) {
        ctx.init();
        if let Event::AnimFrame(interval) = event {
            self.t += (*interval as f64) * 1e-9;
            if self.t >= 1.0 {
                self.t = 0.0;
            }
            ctx.request_anim_frame();
            ctx.request_paint();
        }
    }

    fn on_status_change(&mut self, _ctx: &mut LifeCycleCtx, _event: &StatusChange, _env: &Env) {}

    fn lifecycle(&mut self, ctx: &mut LifeCycleCtx, event: &LifeCycle, _env: &Env) {
        ctx.init();
        if let LifeCycle::WidgetAdded = event {
            ctx.request_anim_frame();
            ctx.request_paint();
        }
    }

    fn layout(&mut self, ctx: &mut LayoutCtx, bc: &BoxConstraints, env: &Env) -> Size {
        ctx.init();
        let size = if bc.is_width_bounded() && bc.is_height_bounded() {
            bc.max()
        } else {
            bc.constrain(Size::new(
                env.get(theme::BASIC_WIDGET_HEIGHT),
                env.get(theme::BASIC_WIDGET_HEIGHT),
            ))
        };

        trace!("Computed size: {}", size);
        size
    }

    fn paint(&mut self, ctx: &mut PaintCtx, env: &Env) {
        ctx.init();
        let t = self.t;
        let (width, height) = (ctx.size().width, ctx.size().height);
        let center = Point::new(width / 2.0, height / 2.0);
        let (r, g, b, original_alpha) = Color::as_rgba(&self.color.resolve(env));
        let scale_factor = width.min(height) / 40.0;

        for step in 1..=12 {
            let step = f64::from(step);
            let fade_t = (t * 12.0 + 1.0).trunc();
            let fade = ((fade_t + step).rem_euclid(12.0) / 12.0) + 1.0 / 12.0;
            let angle = Vec2::from_angle((step / 12.0) * -2.0 * PI);
            let ambit_start = center + (10.0 * scale_factor * angle);
            let ambit_end = center + (20.0 * scale_factor * angle);
            let color = Color::rgba(r, g, b, fade * original_alpha);

            ctx.stroke(
                Line::new(ambit_start, ambit_end),
                &color,
                3.0 * scale_factor,
            );
        }
    }

    fn children(&self) -> SmallVec<[WidgetRef<'_, dyn Widget>; 16]> {
        SmallVec::new()
    }
}
