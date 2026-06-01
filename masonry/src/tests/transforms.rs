// Copyright 2025 the Xilem Authors and the Druid Authors
// SPDX-License-Identifier: Apache-2.0

//! Tests related to transforms.

use core::f64::consts::PI;

use crate::core::{NewWidget, PropertySet, Widget, WidgetTag};
use crate::kurbo::{Affine, Point, Vec2};
use crate::layout::{AsUnit, SizeDef, UnitPoint};
use crate::peniko::color::palette;
use crate::properties::{Background, BorderColor, BorderWidth, Dimensions, Padding};
use crate::testing::{ModularWidget, TestHarness, WrapperWidget, assert_render_snapshot};
use crate::tests::{assert_point_approx_eq, assert_vec2_approx_eq};
use crate::theme::test_property_set;
use crate::widgets::{Button, ChildAlignment, Label, SizedBox, ZStack};

fn blue_box(inner: impl Widget) -> impl Widget {
    let mut box_props = PropertySet::new();
    box_props.insert(Background::Color(palette::css::BLUE));
    box_props.insert(BorderColor::new(palette::css::TEAL));
    box_props.insert(BorderWidth::all(2.px()));

    WrapperWidget::new(
        NewWidget::new(
            SizedBox::new(inner.prepare())
                .width(100.px())
                .height(50.px()),
        )
        .with_props(box_props),
    )
}

#[test]
fn transforms_translation_rotation() {
    let translation = Vec2::new(100.0, 50.0);
    let transformed_widget = NewWidget::new(blue_box(Label::new("Background"))).with_transform(
        // Currently there's no support for changing the transform-origin,
        // which is currently at the top left.
        // This rotates around the center of the widget
        Affine::translate(-translation)
            .then_rotate(PI * 0.25)
            .then_translate(translation),
    );
    let widget = ZStack::new()
        .with(transformed_widget, ChildAlignment::ParentAligned)
        .prepare();

    let mut harness = TestHarness::create(test_property_set(), widget);

    assert_render_snapshot!(harness, "transforms_translation_rotation");
}

#[test]
fn transforms_pointer_events() {
    let transformed_widget = NewWidget::new(blue_box(
        ZStack::new().with(Button::with_text("OK").prepare(), UnitPoint::BOTTOM_RIGHT),
    ))
    .with_transform(Affine::rotate(PI * 0.125).then_translate(Vec2::new(100.0, 50.0)));
    let widget = ZStack::new()
        .with(transformed_widget, ChildAlignment::ParentAligned)
        .prepare();

    let mut harness = TestHarness::create(test_property_set(), widget);

    harness.mouse_move((300.0, 280.0)); // Should hit the "O" of the button text
    harness.mouse_button_press(None);

    assert_render_snapshot!(harness, "transforms_pointer_events");
}

#[test]
fn transforms_handle_content_box_space_translation() {
    let tag = WidgetTag::unique();
    let child = NewWidget::new(SizedBox::empty().size(10.px(), 8.px()))
        .with_tag(tag)
        .with_transform(Affine::scale_non_uniform(2., 3.))
        .with_props((
            BorderWidth::all(0.5.px()),
            Padding {
                left: 1.px(),
                right: 0.5.px(),
                top: 2.px(),
                bottom: 1.5.px(),
            },
        ));

    let root = ModularWidget::new_parent(child)
        .layout_fn(|child, ctx, _, size| {
            let child_size = ctx.compute_size(child, SizeDef::fit(size), size.into());
            ctx.run_layout(child, child_size);
            ctx.place_child(child, Point::new(5., 7.));
        })
        .prepare()
        .with_props(Dimensions::fixed(40.px(), 40.px()));

    let harness = TestHarness::create(test_property_set(), root);
    let child = harness.get_widget(tag);
    let ctx = child.ctx();

    // Border 0.5 + padding (1.0,2.0) gives top-left content inset (1.5,2.5).
    assert_vec2_approx_eq(
        "border_box_translation",
        ctx.border_box_translation(),
        Vec2::new(1.5, 2.5),
    );
    // (0,0) + content inset (1.5,2.5), then scale (2,3) and add layout origin (5,7).
    assert_point_approx_eq(
        "to_window content origin",
        ctx.to_window(Point::ORIGIN),
        Point::new(8., 14.5),
    );
    // (2,1) + content inset (1.5,2.5) = (3.5,3.5), then scale (2,3) and add (5,7).
    assert_point_approx_eq(
        "to_window local point",
        ctx.to_window(Point::new(2., 1.)),
        Point::new(12., 17.5),
    );
    // Border box origin (-1.5,-2.5) cancels content inset, leaving the layout origin (5,7).
    assert_point_approx_eq(
        "to_window border origin",
        ctx.to_window(ctx.border_box().origin()),
        Point::new(5., 7.),
    );
    // Inverse: ((12,17.5) - (5,7)) / (2,3) = (3.5,3.5), then subtract inset (1.5,2.5).
    assert_point_approx_eq(
        "to_local",
        ctx.to_local(Point::new(12., 17.5)),
        Point::new(2., 1.),
    );
    // window_transform bakes in the required calculations and achieves the same result.
    assert_point_approx_eq(
        "window_transform",
        ctx.window_transform() * Point::new(2., 1.),
        Point::new(12., 17.5),
    );
}
