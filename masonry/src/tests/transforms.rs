// Copyright 2025 the Xilem Authors and the Druid Authors
// SPDX-License-Identifier: Apache-2.0

//! Tests related to transforms.

use core::f64::consts::PI;

use crate::core::{NewWidget, PropertySet, Widget, WidgetTag};
use crate::kurbo::{Affine, Point, Rect, Size, Vec2};
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
            ctx.layout_child(child, Point::new(5., 7.), child_size);
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

#[test]
fn unsupported_transform_disables_pixel_snap() {
    let child_tag = WidgetTag::unique();
    let child = NewWidget::new(SizedBox::empty().size(10.3.px(), 10.3.px())).with_tag(child_tag);
    let parent = ModularWidget::new_parent(child).layout_fn(move |child, ctx, _, size| {
        let child_size = ctx.compute_size(child, SizeDef::fit(size), size.into());
        ctx.layout_child(child, Point::new(5.1, 5.3), child_size);
    });
    let parent = NewWidget::new(parent).with_transform(Affine::rotate(0.25));

    let harness = TestHarness::create(test_property_set(), parent);

    let child = harness.get_widget(child_tag);
    let ctx = child.ctx();

    assert_eq!(ctx.border_box().size(), Size::new(10.3, 10.3));
}

#[test]
fn pixel_snapping_after_window_transforms() {
    #[track_caller]
    fn assert_integer_edges(name: &str, rect: Rect) {
        let edges = [rect.x0, rect.y0, rect.x1, rect.y1];
        assert!(
            edges.iter().all(|edge| (edge - edge.round()).abs() < 1e-9),
            "{name}: expected integer edges, got {rect:?}"
        );
    }

    let translated_tag = WidgetTag::unique();
    let scaled_tag = WidgetTag::unique();
    let flipped_tag = WidgetTag::unique();
    let nested_tag = WidgetTag::unique();

    let translated = NewWidget::new(SizedBox::empty().size(12.2.px(), 8.4.px()))
        .with_tag(translated_tag)
        .with_transform(Affine::translate(Vec2::new(0.37, 0.61)))
        .erased();
    let scaled = NewWidget::new(SizedBox::empty().size(9.3.px(), 11.7.px()))
        .with_tag(scaled_tag)
        .with_transform(Affine::scale_non_uniform(1.25, 0.8).then_translate(Vec2::new(0.41, 0.29)))
        .erased();
    let flipped = NewWidget::new(SizedBox::empty().size(10.6.px(), 7.5.px()))
        .with_tag(flipped_tag)
        .with_transform(
            Affine::scale_non_uniform(-0.75, 1.4).then_translate(Vec2::new(0.48, -0.33)),
        )
        .erased();
    let nested = NewWidget::new(SizedBox::empty().size(8.2.px(), 6.6.px()))
        .with_tag(nested_tag)
        .with_transform(Affine::scale_non_uniform(0.6, 1.35).then_translate(Vec2::new(0.27, 0.43)))
        .erased();

    let inner = ModularWidget::new_parent(nested)
        .layout_fn(|child, ctx, _, size| {
            let child_size = ctx.compute_size(child, SizeDef::fit(size), size.into());
            ctx.layout_child(child, Point::new(1.7, 2.2), child_size);
        })
        .compose_fn(|child, ctx| {
            ctx.set_child_scroll_translation(child, Vec2::new(0.33, -0.47));
        });
    let inner = NewWidget::new(inner)
        .with_transform(Affine::scale_non_uniform(1.5, -0.9).then_translate(Vec2::new(0.19, 0.71)))
        .erased();

    let outer = ModularWidget::new_parent(inner)
        .layout_fn(|child, ctx, _, size| {
            let child_size = ctx.compute_size(child, SizeDef::fit(size), size.into());
            ctx.layout_child(child, Point::new(4.6, 3.9), child_size);
        })
        .compose_fn(|child, ctx| {
            ctx.set_child_scroll_translation(child, Vec2::new(-0.22, 0.35));
        });
    let outer = NewWidget::new(outer)
        .with_transform(Affine::scale_non_uniform(-1.2, 0.7).then_translate(Vec2::new(0.52, -0.24)))
        .erased();

    let positions = [
        Point::new(2.3, 4.7),
        Point::new(19.4, 3.6),
        Point::new(37.8, 8.2),
        Point::new(57.1, 5.4),
    ];
    let scroll_offsets = [
        Vec2::new(0.21, 0.36),
        Vec2::new(-0.44, 0.52),
        Vec2::new(0.68, -0.17),
        Vec2::new(-0.31, 0.49),
    ];

    let root = ModularWidget::new(vec![
        translated.to_pod(),
        scaled.to_pod(),
        flipped.to_pod(),
        outer.to_pod(),
    ])
    .layout_fn(move |children, ctx, _, size| {
        for (idx, child) in children.iter_mut().enumerate() {
            let child_size = ctx.compute_size(child, SizeDef::fit(size), size.into());
            ctx.layout_child(child, positions[idx], child_size);
        }
    })
    .compose_fn(move |children, ctx| {
        for (idx, child) in children.iter_mut().enumerate() {
            ctx.set_child_scroll_translation(child, scroll_offsets[idx]);
        }
    })
    .register_children_fn(|children, ctx| {
        for child in children {
            ctx.register_child(child);
        }
    })
    .children_fn(|children| children.iter().map(|child| child.id()).collect())
    .prepare();

    let harness = TestHarness::create_with_size(test_property_set(), root, (200, 120));

    let assert_pixel_aligned = |name: &str, tag: WidgetTag<SizedBox>| {
        let widget = harness.get_widget(tag);
        let ctx = widget.ctx();
        let window_border_box = ctx.window_transform().transform_rect_bbox(ctx.border_box());

        assert_integer_edges(name, window_border_box);
    };

    assert_pixel_aligned("translated", translated_tag);
    assert_pixel_aligned("scaled", scaled_tag);
    assert_pixel_aligned("flipped", flipped_tag);
    assert_pixel_aligned("nested", nested_tag);
}
