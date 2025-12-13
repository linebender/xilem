// Copyright 2025 the Xilem Authors
// SPDX-License-Identifier: Apache-2.0

use assert_matches::assert_matches;
use masonry_core::core::{NewWidget, WidgetTag};
use masonry_core::palette::css::{BLUE, GREEN, RED};
use masonry_core::util::{fill, stroke};
use masonry_testing::{ModularWidget, Record, TestHarness, TestWidgetExt, assert_render_snapshot};
use vello::kurbo::{Affine, Circle, Dashes, Point, Size, Stroke, Vec2};
use vello::peniko::Color;

use crate::properties::Background;
use crate::properties::types::{Length, MainAxisAlignment};
use crate::theme::test_property_set;
use crate::widgets::{Flex, SizedBox};

#[test]
fn request_paint() {
    let target_tag = WidgetTag::named("target");
    let parent_tag = WidgetTag::named("parent");
    let child = NewWidget::new_with_tag(SizedBox::empty().record(), target_tag);
    let parent = NewWidget::new_with_tag(ModularWidget::new_parent(child).record(), parent_tag);
    let grandparent = NewWidget::new(ModularWidget::new_parent(parent));

    let mut harness = TestHarness::create(test_property_set(), grandparent);
    let _ = harness.render();
    harness.flush_records_of(target_tag);
    harness.flush_records_of(parent_tag);

    // Paint
    harness.edit_widget(target_tag, |mut widget| {
        widget.ctx.request_paint_only();
    });
    let _ = harness.render();

    // Check that `Widget::paint()` is called for the child (which did request it)
    // but not the parent (which did not).
    assert_matches!(harness.take_records_of(target_tag)[..], [Record::Paint]);
    assert_matches!(harness.take_records_of(parent_tag)[..], []);

    // Post-paint
    harness.edit_widget(target_tag, |mut widget| {
        widget.ctx.request_post_paint();
    });
    let _ = harness.render();

    // Check that `Widget::post_paint()` is called for the child (which did request it)
    // but not the parent (which did not).
    assert_matches!(harness.take_records_of(target_tag)[..], [Record::PostPaint]);
    assert_matches!(harness.take_records_of(parent_tag)[..], []);

    // Check that `Widget::paint()` and `Widget::post_paint()` are not called:
    // neither widget has requested an update.
    let _ = harness.render();
    assert_matches!(harness.take_records_of(parent_tag)[..], []);
    assert_matches!(harness.take_records_of(target_tag)[..], []);
}

#[test]
fn paint_order() {
    const SQUARE_SIZE: f64 = 30.;
    const SQUARE_LENGTH: Length = Length::const_px(SQUARE_SIZE);
    let child1 = NewWidget::new_with_props(
        SizedBox::empty().width(SQUARE_LENGTH).height(SQUARE_LENGTH),
        (Background::Color(RED),).into(),
    );
    let child2 = NewWidget::new_with_props(
        SizedBox::empty().width(SQUARE_LENGTH).height(SQUARE_LENGTH),
        (Background::Color(GREEN),).into(),
    );
    let child3 = NewWidget::new_with_props(
        SizedBox::empty().width(SQUARE_LENGTH).height(SQUARE_LENGTH),
        (Background::Color(BLUE),).into(),
    );
    let children = vec![child1, child2, child3];
    let parent = NewWidget::new(
        ModularWidget::new_multi_parent(children)
            .layout_fn(move |children, ctx, _props, bc| {
                let mut pos = Point::ZERO;
                for child in children {
                    let _ = ctx.run_layout(child, bc);

                    ctx.place_child(child, pos);
                    pos += Vec2::new(SQUARE_SIZE / 2., SQUARE_SIZE / 2.);
                }
                Size::new(SQUARE_SIZE * 2., SQUARE_SIZE * 2.)
            })
            .paint_fn(|_, ctx, _, scene| {
                fill(scene, &ctx.size().to_rect(), Color::WHITE);
            })
            .post_paint_fn(|_, ctx, _, scene| {
                let rect = ctx.size().to_rect().inset(-0.5);
                stroke(scene, &rect, Color::BLACK, 1.0);
            }),
    );
    let grandparent = NewWidget::new(
        Flex::column()
            .main_axis_alignment(MainAxisAlignment::Center)
            .with_fixed(parent),
    );

    let mut harness = TestHarness::create_with_size(
        test_property_set(),
        grandparent,
        Size::new(SQUARE_SIZE * 3., SQUARE_SIZE * 3.),
    );

    // The resulting image should have, from background to foreground:
    // - The harness's default color background.
    // - The parent's white square.
    // - The red, green, then blue square.
    // - The parent's post-paint black square.
    assert_render_snapshot!(harness, "paint_order");
}

#[test]
fn paint_clipping() {
    const SQUARE_SIZE: f64 = 80.;

    let circle = Circle::new((SQUARE_SIZE / 2., SQUARE_SIZE / 2.), SQUARE_SIZE * 0.60);

    let parent = NewWidget::new(
        ModularWidget::new(())
            .layout_fn(|_, ctx, _, _| {
                let size = Size::new(SQUARE_SIZE, SQUARE_SIZE);
                ctx.set_clip_path(size.to_rect());
                size
            })
            .paint_fn(move |_, ctx, _, scene| {
                fill(scene, &ctx.size().to_rect(), Color::WHITE);
                fill(scene, &circle, RED);
            })
            .post_paint_fn(move |_, _, _, scene| {
                let style = Stroke {
                    width: 4.0,
                    dash_pattern: Dashes::from_slice(&[12.0, 12.0]),
                    ..Default::default()
                };
                scene.stroke(&style, Affine::IDENTITY, Color::BLACK, None, &circle);
            }),
    );
    let parent = NewWidget::new(
        Flex::column()
            .main_axis_alignment(MainAxisAlignment::Center)
            .with_fixed(parent),
    );

    let mut harness = TestHarness::create_with_size(
        test_property_set(),
        parent,
        Size::new(SQUARE_SIZE * 2., SQUARE_SIZE * 2.),
    );

    // The red circle should be clipped by the square.
    // The dashed circle shouldn't.
    assert_render_snapshot!(harness, "paint_clipping");
}
