// This software is licensed under Apache License 2.0 and distributed on an
// "as-is" basis without warranties of any kind. See the LICENSE file for
// details.

//! An animated spinner widget.

use std::f64::consts::PI;

use smallvec::SmallVec;
use tracing::trace;

use crate::kurbo::Line;
use crate::widget::WidgetRef;
use crate::{
    theme, BoxConstraints, Color, Event, EventCtx, LayoutCtx, LifeCycle, LifeCycleCtx, PaintCtx,
    Point, RenderContext, Size, StatusChange, Vec2, Widget,
};

// TODO - Set color
/// An animated spinner widget for showing a loading state.
///
/// To customize the spinner's size, you can place it inside a [`SizedBox`]
/// that has a fixed width and height.
///
/// [`SizedBox`]: struct.SizedBox.html
pub struct Spinner {
    t: f64,
    color: Color,
}

crate::declare_widget!(SpinnerMut, Spinner);

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
    pub fn with_color(mut self, color: impl Into<Color>) -> Self {
        self.color = color.into();
        self
    }
}

impl SpinnerMut<'_, '_> {
    /// Set the spinner's color.
    ///
    /// The argument can be either a `Color` or a [`Key<Color>`].
    ///
    /// [`Key<Color>`]: ../struct.Key.html
    pub fn set_color(&mut self, color: impl Into<Color>) {
        self.widget.color = color.into();
        self.ctx.request_paint();
    }
}

impl Default for Spinner {
    fn default() -> Self {
        Spinner {
            t: 0.0,
            color: theme::TEXT_COLOR,
        }
    }
}

impl Widget for Spinner {
    fn on_event(&mut self, ctx: &mut EventCtx, event: &Event) {
        if let Event::AnimFrame(interval) = event {
            self.t += (*interval as f64) * 1e-9;
            if self.t >= 1.0 {
                self.t = 0.0;
            }
            ctx.request_anim_frame();
            ctx.request_paint();
        }
    }

    fn on_status_change(&mut self, _ctx: &mut LifeCycleCtx, _event: &StatusChange) {}

    fn lifecycle(&mut self, ctx: &mut LifeCycleCtx, event: &LifeCycle) {
        if let LifeCycle::WidgetAdded = event {
            ctx.request_anim_frame();
            ctx.request_paint();
        }
    }

    fn layout(&mut self, _ctx: &mut LayoutCtx, bc: &BoxConstraints) -> Size {
        let size = if bc.is_width_bounded() && bc.is_height_bounded() {
            bc.max()
        } else {
            bc.constrain(Size::new(
                theme::BASIC_WIDGET_HEIGHT,
                theme::BASIC_WIDGET_HEIGHT,
            ))
        };

        trace!("Computed size: {}", size);
        size
    }

    fn paint(&mut self, ctx: &mut PaintCtx) {
        let t = self.t;
        let (width, height) = (ctx.size().width, ctx.size().height);
        let center = Point::new(width / 2.0, height / 2.0);
        let (r, g, b, original_alpha) = Color::as_rgba(self.color);
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

#[cfg(test)]
mod tests {
    use super::*;
    use crate::assert_render_snapshot;
    use crate::testing::TestHarness;
    //use instant::Duration;

    #[test]
    fn simple_spinner() {
        let spinner = Spinner::new();

        let mut harness = TestHarness::create(spinner);
        assert_render_snapshot!(harness, "spinner_init");

        // TODO - See issue #12
        //harness.move_timers_forward(Duration::from_millis(700));
        //assert_render_snapshot!(harness, "spinner_700ms");
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
                let mut spinner = spinner.downcast::<Spinner>().unwrap();
                spinner.set_color(Color::PURPLE);
            });

            harness.render()
        };

        // We don't use assert_eq because we don't want rich assert
        assert!(image_1 == image_2);
    }
}
