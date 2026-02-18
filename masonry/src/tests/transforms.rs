// Copyright 2025 the Xilem Authors and the Druid Authors
// SPDX-License-Identifier: Apache-2.0

//! Tests related to transforms.

use std::f64::consts::PI;

use masonry_testing::WrapperWidget;

use crate::core::{NewWidget, PointerButton, PropertySet, Widget, WidgetOptions};
use crate::kurbo::{Affine, Vec2};
use crate::layout::AsUnit;
use crate::peniko::color::palette;
use crate::properties::types::UnitPoint;
use crate::properties::{Background, BorderColor, BorderWidth};
use crate::testing::{TestHarness, assert_render_snapshot};
use crate::theme::default_property_set;
use crate::widgets::{Button, ChildAlignment, Label, SizedBox, ZStack};

fn blue_box(inner: impl Widget) -> impl Widget {
    let mut box_props = PropertySet::new();
    box_props.insert(Background::Color(palette::css::BLUE));
    box_props.insert(BorderColor::new(palette::css::TEAL));
    box_props.insert(BorderWidth::all(2.0));

    WrapperWidget::new(
        SizedBox::new(inner.with_auto_id())
            .width(200.px())
            .height(100.px())
            .with_props(box_props),
    )
}

#[test]
fn transforms_translation_rotation() {
    let translation = Vec2::new(100.0, 50.0);
    let transformed_widget = NewWidget::new_with_options(
        blue_box(Label::new("Background")),
        // Currently there's no support for changing the transform-origin, which is currently at the top left.
        // This rotates around the center of the widget
        WidgetOptions {
            transform: Affine::translate(-translation)
                .then_rotate(PI * 0.25)
                .then_translate(translation),
            ..Default::default()
        },
    );
    let widget = ZStack::new()
        .with_fixed(transformed_widget, ChildAlignment::ParentAligned)
        .with_auto_id();

    let mut harness = TestHarness::create(default_property_set(), widget);
    assert_render_snapshot!(harness, "transforms_translation_rotation");
}

#[test]
fn transforms_pointer_events() {
    let transformed_widget = NewWidget::new_with_options(
        blue_box(ZStack::new().with_fixed(
            Button::with_text("Should be pressed").with_auto_id(),
            UnitPoint::BOTTOM_RIGHT,
        )),
        WidgetOptions {
            transform: Affine::rotate(PI * 0.125).then_translate(Vec2::new(100.0, 50.0)),
            ..Default::default()
        },
    );
    let widget = ZStack::new()
        .with_fixed(transformed_widget, ChildAlignment::ParentAligned)
        .with_auto_id();

    let mut harness = TestHarness::create(default_property_set(), widget);
    harness.mouse_move((335.0, 350.0)); // Should hit the last "d" of the button text
    harness.mouse_button_press(PointerButton::Primary);
    assert_render_snapshot!(harness, "transforms_pointer_events");
}
