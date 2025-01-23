// Copyright 2025 the Xilem Authors and the Druid Authors
// SPDX-License-Identifier: Apache-2.0

//! Tests related to transforms.

use std::f64::consts::PI;

use vello::kurbo::Affine;
use vello::kurbo::Vec2;
use vello::peniko::color::palette;

use crate::assert_render_snapshot;
use crate::testing::TestHarness;
use crate::widgets::Alignment;
use crate::widgets::Button;
use crate::widgets::ChildAlignment;
use crate::widgets::Label;
use crate::widgets::SizedBox;
use crate::widgets::ZStack;
use crate::core::PointerButton;
use crate::core::Widget;
use crate::core::WidgetPod;

fn blue_box(inner: impl Widget) -> Box<SizedBox> {
    Box::new(
        SizedBox::new(inner)
            .width(200.)
            .height(100.)
            .background(palette::css::BLUE)
            .border(palette::css::TEAL, 2.),
    )
}

#[test]
fn transforms_translation_rotation() {
    let translation = Vec2::new(100.0, 50.0);
    let transformed_widget = WidgetPod::new_with_transform(
        blue_box(Label::new("Background")),
        // Currently there's no support for changing the transform-origin, which is currently at the top left.
        // This rotates around the center of the widget
        Affine::translate(-translation)
            .then_rotate(PI * 0.25)
            .then_translate(translation),
    )
    .erased();
    let widget = ZStack::new().with_child_pod(transformed_widget, ChildAlignment::ParentAligned);

    let mut harness = TestHarness::create(widget);
    assert_render_snapshot!(harness, "transforms_translation_rotation");
}

#[test]
fn transforms_pointer_events() {
    let transformed_widget = WidgetPod::new_with_transform(
        blue_box(
            ZStack::new().with_child(Button::new("Should be pressed"), Alignment::BottomTrailing),
        ),
        Affine::rotate(PI * 0.125).then_translate(Vec2::new(100.0, 50.0)),
    )
    .erased();
    let widget = ZStack::new().with_child_pod(transformed_widget, ChildAlignment::ParentAligned);

    let mut harness = TestHarness::create(widget);
    harness.mouse_move((335.0, 350.0)); // Should hit the last "d" of the button text
    harness.mouse_button_press(PointerButton::Primary);
    assert_render_snapshot!(harness, "transforms_pointer_events");
}
