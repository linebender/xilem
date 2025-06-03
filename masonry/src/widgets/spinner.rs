// Copyright 2020 the Xilem Authors and the Druid Authors
// SPDX-License-Identifier: Apache-2.0

//! An animated spinner widget.

use std::f64::consts::PI;

use accesskit::{Node, Role};
use smallvec::SmallVec;
use tracing::{Span, trace_span};
use vello::Scene;
use vello::kurbo::{Affine, Cap, Line, Stroke};

use crate::core::{
    AccessCtx, AccessEvent, BoxConstraints, EventCtx, LayoutCtx, PaintCtx, PointerEvent,
    PropertiesMut, PropertiesRef, QueryCtx, RegisterCtx, TextEvent, Update, UpdateCtx, Widget,
    WidgetId, WidgetMut,
};
use crate::kurbo::{Point, Size, Vec2};
use crate::peniko::Color;
use crate::theme;

/// An animated spinner widget for showing a loading state.
///
/// To customize the spinner's size, you can place it inside a [`SizedBox`]
/// that has a fixed width and height.
///
/// [`SizedBox`]: crate::widgets::SizedBox
///
#[doc = crate::include_screenshot!("spinner_init.png", "Spinner frame.")]
pub struct Spinner {
    t: f64,
    color: Color,
}

// --- MARK: BUILDERS
impl Spinner {
    /// Create a spinner widget
    pub fn new() -> Self {
        Self::default()
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
        Self {
            t: 0.0,
            color: DEFAULT_SPINNER_COLOR,
        }
    }
}

// --- MARK: WIDGETMUT
impl Spinner {
    /// Set the spinner's color.
    pub fn set_color(this: &mut WidgetMut<'_, Self>, color: impl Into<Color>) {
        this.widget.color = color.into();
        this.ctx.request_paint_only();
    }

    /// Reset the spinner's color to its default value.
    pub fn reset_color(this: &mut WidgetMut<'_, Self>) {
        Self::set_color(this, DEFAULT_SPINNER_COLOR);
    }
}

// --- MARK: IMPL WIDGET
impl Widget for Spinner {
    fn on_pointer_event(
        &mut self,
        _ctx: &mut EventCtx,
        _props: &mut PropertiesMut<'_>,
        _event: &PointerEvent,
    ) {
    }

    fn on_text_event(
        &mut self,
        _ctx: &mut EventCtx,
        _props: &mut PropertiesMut<'_>,
        _event: &TextEvent,
    ) {
    }

    fn on_access_event(
        &mut self,
        _ctx: &mut EventCtx,
        _props: &mut PropertiesMut<'_>,
        _event: &AccessEvent,
    ) {
    }

    fn on_anim_frame(
        &mut self,
        ctx: &mut UpdateCtx,
        _props: &mut PropertiesMut<'_>,
        interval: u64,
    ) {
        self.t += (interval as f64) * 1e-9;
        if self.t >= 1.0 {
            self.t = self.t.rem_euclid(1.0);
        }
        ctx.request_anim_frame();
        ctx.request_paint_only();
    }

    fn register_children(&mut self, _ctx: &mut RegisterCtx) {}

    fn update(&mut self, ctx: &mut UpdateCtx, _props: &mut PropertiesMut<'_>, event: &Update) {
        match event {
            Update::WidgetAdded => {
                ctx.request_anim_frame();
            }
            _ => (),
        }
    }

    fn layout(
        &mut self,
        _ctx: &mut LayoutCtx,
        _props: &mut PropertiesMut<'_>,
        bc: &BoxConstraints,
    ) -> Size {
        if bc.is_width_bounded() && bc.is_height_bounded() {
            bc.max()
        } else {
            bc.constrain(Size::new(
                theme::BASIC_WIDGET_HEIGHT,
                theme::BASIC_WIDGET_HEIGHT,
            ))
        }
    }

    fn paint(&mut self, ctx: &mut PaintCtx, _props: &PropertiesRef<'_>, scene: &mut Scene) {
        let t = self.t;
        let (width, height) = (ctx.size().width, ctx.size().height);
        let center = Point::new(width / 2.0, height / 2.0);
        let scale_factor = width.min(height) / 40.0;

        for step in 1..=12 {
            let step = f64::from(step);
            let fade_t = (t * 12.0 + 1.0).trunc();
            let fade = ((fade_t + step).rem_euclid(12.0) / 12.0) + 1.0 / 12.0;
            let angle = Vec2::from_angle((step / 12.0) * -2.0 * PI);
            let ambit_start = center + (10.0 * scale_factor * angle);
            let ambit_end = center + (20.0 * scale_factor * angle);
            let color = self.color.multiply_alpha(fade as f32);

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
        Role::ProgressIndicator
    }

    fn accessibility(
        &mut self,
        _ctx: &mut AccessCtx,
        _props: &PropertiesRef<'_>,
        _node: &mut Node,
    ) {
    }

    fn children_ids(&self) -> SmallVec<[WidgetId; 16]> {
        SmallVec::new()
    }

    fn make_trace_span(&self, ctx: &QueryCtx<'_>) -> Span {
        trace_span!("Spinner", id = ctx.widget_id().trace())
    }
}

// --- MARK: TESTS
#[cfg(test)]
mod tests {
    use super::*;
    use crate::palette;
    use crate::testing::{TestHarness, assert_render_snapshot};
    use crate::theme::default_property_set;

    #[test]
    fn simple_spinner() {
        let spinner = Spinner::new();

        let window_size = Size::new(100.0, 100.0);
        let mut harness =
            TestHarness::create_with_size(default_property_set(), spinner, window_size);
        assert_render_snapshot!(harness, "spinner_init");

        harness.animate_ms(700);
        assert_render_snapshot!(harness, "spinner_700ms");

        harness.animate_ms(400);
        assert_render_snapshot!(harness, "spinner_1100ms");
    }

    #[test]
    fn edit_spinner() {
        let image_1 = {
            let spinner = Spinner::new().with_color(palette::css::PURPLE);

            let mut harness = TestHarness::create_with_size(
                default_property_set(),
                spinner,
                Size::new(30.0, 30.0),
            );
            harness.render()
        };

        let image_2 = {
            let spinner = Spinner::new();

            let mut harness = TestHarness::create_with_size(
                default_property_set(),
                spinner,
                Size::new(30.0, 30.0),
            );

            harness.edit_root_widget(|mut spinner| {
                let mut spinner = spinner.downcast::<Spinner>();
                Spinner::set_color(&mut spinner, palette::css::PURPLE);
            });

            harness.render()
        };

        // We don't use assert_eq because we don't want rich assert
        assert!(image_1 == image_2);
    }
}
