// Copyright 2025 the Xilem Authors
// SPDX-License-Identifier: Apache-2.0

use assert_matches::assert_matches;

use crate::core::{NewWidget, PropertySet, Widget, WidgetTag};
use crate::kurbo::{Affine, Circle, Dashes, Point, Size, Stroke, Vec2};
use crate::layout::{AsUnit, Length, SizeDef, UnitPoint};
use crate::palette::css::{BLUE, GREEN, RED};
use crate::peniko::Color;
use crate::peniko::color::{AlphaColor, Srgb};
use crate::properties::types::MainAxisAlignment;
use crate::properties::{Background, Dimensions, Gap, Padding};
use crate::testing::{ModularWidget, Record, TestHarness, TestWidgetExt, assert_render_snapshot};
use crate::theme::test_property_set;
use crate::util::{fill, stroke};
use crate::widgets::{Align, ChildAlignment, Flex, Grid, GridParams, Label, SizedBox, ZStack};

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
        Background::Color(RED),
    );
    let child2 = NewWidget::new_with_props(
        SizedBox::empty().width(SQUARE_LENGTH).height(SQUARE_LENGTH),
        Background::Color(GREEN),
    );
    let child3 = NewWidget::new_with_props(
        SizedBox::empty().width(SQUARE_LENGTH).height(SQUARE_LENGTH),
        Background::Color(BLUE),
    );
    let children = vec![child1, child2, child3];
    let parent = NewWidget::new(
        ModularWidget::new_multi_parent(children)
            .measure_fn(|_, _, _, _, _, _| SQUARE_SIZE * 2.)
            .layout_fn(move |children, ctx, _props, size| {
                let mut pos = Point::ZERO;
                for child in children {
                    let child_size = ctx.compute_size(child, SizeDef::fit(size), size.into());
                    ctx.run_layout(child, child_size);
                    ctx.place_child(child, pos);
                    pos += Vec2::new(SQUARE_SIZE / 2., SQUARE_SIZE / 2.);
                }
            })
            .paint_fn(|_, ctx, _, scene| {
                fill(scene, &ctx.content_box(), Color::WHITE);
            })
            .post_paint_fn(|_, ctx, _, scene| {
                let rect = ctx.content_box().inset(-0.5);
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
            .measure_fn(|_, _, _, _, _, _| SQUARE_SIZE)
            .layout_fn(|_, ctx, _, size| {
                ctx.set_clip_path(size.to_rect());
            })
            .paint_fn(move |_, ctx, _, scene| {
                fill(scene, &ctx.content_box(), Color::WHITE);
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

// Layered slightly misaligned grid layer painting:
//
// Color background
// [No bg A] [0.0 bg A] [0.5 bg A] [1.0 bg A]
//   [No bg B] [0.0 bg B] [0.5 bg B] [1.0 bg B]
#[test]
fn paint_transparency() {
    fn child(
        text: &str,
        align: UnitPoint,
        bg_color: impl Into<Option<AlphaColor<Srgb>>>,
    ) -> NewWidget<Align> {
        let bg_color = bg_color.into();

        let label = Label::new(text).with_props((
            Background::Color(Color::TRANSPARENT),
            Dimensions::fixed(20.px(), 20.px()),
        ));

        let mut props = PropertySet::new();
        if let Some(bg_color) = bg_color {
            props = props.with(Background::Color(bg_color));
        }

        Align::new(align, label).with_props(props)
    }

    let align_a = UnitPoint::TOP_LEFT;
    let align_b = UnitPoint::BOTTOM_LEFT;

    let mut grid_a = Grid::with_dimensions(4, 1);
    grid_a = grid_a.with(child("AAAA", align_a, None), GridParams::new(0, 0, 1, 1));
    grid_a = grid_a.with(
        child("AABB", align_a, Color::TRANSPARENT),
        GridParams::new(1, 0, 1, 1),
    );
    grid_a = grid_a.with(
        child("AACC", align_a, Color::from_rgba8(66, 117, 245, 127)),
        GridParams::new(2, 0, 1, 1),
    );
    // Stupid workaround for typos-cli thinking it's a typo.
    let typo = concat!("AA", "DD");
    grid_a = grid_a.with(
        child(typo, align_a, Color::from_rgba8(66, 117, 245, 255)),
        GridParams::new(3, 0, 1, 1),
    );

    let mut grid_b = Grid::with_dimensions(16, 1);
    grid_b = grid_b.with(child("BBAA", align_b, None), GridParams::new(1, 0, 3, 1));
    grid_b = grid_b.with(
        child("BBBB", align_b, Color::TRANSPARENT),
        GridParams::new(5, 0, 3, 1),
    );
    grid_b = grid_b.with(
        child("BBCC", align_b, Color::from_rgba8(245, 66, 191, 127)),
        GridParams::new(9, 0, 3, 1),
    );
    grid_b = grid_b.with(
        child("BBDD", align_b, Color::from_rgba8(245, 66, 191, 255)),
        GridParams::new(13, 0, 3, 1),
    );

    let props = (Padding::all(20.), Gap::new(10.px()));
    let grid_a = grid_a.with_props(props);
    let grid_b = grid_b.with_props(props);

    let mut root = ZStack::new();
    root = root.with(grid_a, ChildAlignment::ParentAligned);
    root = root.with(grid_b, ChildAlignment::ParentAligned);

    let mut harness = TestHarness::create_with_size(
        test_property_set(),
        root.with_auto_id(),
        Size::new(350., 80.),
    );

    assert_render_snapshot!(harness, "paint_transparency");
}
