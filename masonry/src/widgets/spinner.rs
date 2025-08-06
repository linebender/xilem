// Copyright 2020 the Xilem Authors and the Druid Authors
// SPDX-License-Identifier: Apache-2.0

//! An animated spinner widget.

use std::any::TypeId;
use std::f64::consts::PI;

use accesskit::{Node, Role};
use tracing::{Span, trace_span};
use vello::Scene;
use vello::kurbo::{Affine, Cap, Line, Point, Size, Stroke, Vec2};

use crate::core::{
    AccessCtx, BoxConstraints, ChildrenIds, LayoutCtx, NoAction, PaintCtx, PropertiesMut,
    PropertiesRef, RegisterCtx, Update, UpdateCtx, Widget, WidgetId,
};
use crate::properties::ContentColor;
use crate::theme;
use crate::util::include_screenshot;

/// An animated spinner widget for showing a loading state.
///
/// You can customize the look of this spinner with the [`ContentColor`] property.
/// To customize the spinner's size, you can place it inside a [`SizedBox`]
/// that has a fixed width and height.
///
/// [`SizedBox`]: crate::widgets::SizedBox
///
#[doc = include_screenshot!("spinner_init.png", "Spinner frame.")]
pub struct Spinner {
    t: f64,
}

// --- MARK: BUILDERS
impl Spinner {
    /// Create a spinner widget
    pub fn new() -> Self {
        Self::default()
    }
}

impl Default for Spinner {
    fn default() -> Self {
        Self { t: 0.0 }
    }
}

// --- MARK: IMPL WIDGET
impl Widget for Spinner {
    type Action = NoAction;

    fn on_anim_frame(
        &mut self,
        ctx: &mut UpdateCtx<'_>,
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

    fn register_children(&mut self, _ctx: &mut RegisterCtx<'_>) {}

    fn property_changed(&mut self, ctx: &mut UpdateCtx<'_>, property_type: TypeId) {
        ContentColor::prop_changed(ctx, property_type);
    }

    fn update(&mut self, ctx: &mut UpdateCtx<'_>, _props: &mut PropertiesMut<'_>, event: &Update) {
        match event {
            Update::WidgetAdded => {
                ctx.request_anim_frame();
            }
            _ => (),
        }
    }

    fn layout(
        &mut self,
        _ctx: &mut LayoutCtx<'_>,
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

    fn paint(&mut self, ctx: &mut PaintCtx<'_>, props: &PropertiesRef<'_>, scene: &mut Scene) {
        let color = props.get::<ContentColor>();

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
            let color = color.color.multiply_alpha(fade as f32);

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
        _ctx: &mut AccessCtx<'_>,
        _props: &PropertiesRef<'_>,
        _node: &mut Node,
    ) {
    }

    fn children_ids(&self) -> ChildrenIds {
        ChildrenIds::new()
    }

    fn make_trace_span(&self, id: WidgetId) -> Span {
        trace_span!("Spinner", id = id.trace())
    }
}

// --- MARK: TESTS
#[cfg(test)]
mod tests {
    use masonry_core::core::{NewWidget, Properties};

    use super::*;
    use crate::palette;
    use crate::testing::{TestHarness, assert_render_snapshot};
    use crate::theme::default_property_set;

    #[test]
    fn simple_spinner() {
        let spinner = NewWidget::new(Spinner::new());

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
            let spinner =
                Spinner::new().with_props(Properties::one(ContentColor::new(palette::css::PURPLE)));

            let mut harness = TestHarness::create_with_size(
                default_property_set(),
                spinner,
                Size::new(30.0, 30.0),
            );
            harness.render()
        };

        let image_2 = {
            let spinner = NewWidget::new(Spinner::new());

            let mut harness = TestHarness::create_with_size(
                default_property_set(),
                spinner,
                Size::new(30.0, 30.0),
            );

            harness.edit_root_widget(|mut spinner| {
                spinner.insert_prop(ContentColor::new(palette::css::PURPLE));
            });

            harness.render()
        };

        // We don't use assert_eq because we don't want rich assert
        assert!(image_1 == image_2);
    }
}
