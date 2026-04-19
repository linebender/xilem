// Copyright 2025 the Xilem Authors
// SPDX-License-Identifier: Apache-2.0

use std::sync::Arc;

use assert_matches::assert_matches;

use crate::app::{RenderRoot, RenderRootOptions, WindowSizePolicy};
use crate::core::{NewWidget, PaintLayerMode, PropertySet, Widget, WidgetTag};
use crate::dpi::PhysicalSize;
use crate::kurbo::{Circle, Dashes, Point, Stroke, Vec2};
use crate::layout::{AsUnit, Length, SizeDef, UnitPoint};
use crate::palette::css::{BLUE, GREEN, RED};
use crate::peniko::color::{AlphaColor, Srgb};
use crate::peniko::{Blob, Color};
use crate::properties::types::MainAxisAlignment;
use crate::properties::{Background, Dimensions, Gap, Padding};
use crate::testing::{
    ModularWidget, ROBOTO, Record, TestHarness, TestWidgetExt, assert_render_snapshot,
};
use crate::theme::test_property_set;
use crate::widgets::{Align, ChildAlignment, Flex, Grid, GridParams, Label, SizedBox, ZStack};

#[test]
fn request_paint() {
    let target_tag = WidgetTag::named("target");
    let parent_tag = WidgetTag::named("parent");
    let child = NewWidget::new(SizedBox::empty().record()).with_tag(target_tag);
    let parent = NewWidget::new(ModularWidget::new_parent(child).record()).with_tag(parent_tag);
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
    let child1 = NewWidget::new(SizedBox::empty().width(SQUARE_LENGTH).height(SQUARE_LENGTH))
        .with_props(Background::Color(RED));
    let child2 = NewWidget::new(SizedBox::empty().width(SQUARE_LENGTH).height(SQUARE_LENGTH))
        .with_props(Background::Color(GREEN));
    let child3 = NewWidget::new(SizedBox::empty().width(SQUARE_LENGTH).height(SQUARE_LENGTH))
        .with_props(Background::Color(BLUE));
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
                scene.fill(ctx.content_box(), Color::WHITE).draw();
            })
            .post_paint_fn(|_, ctx, _, scene| {
                let rect = ctx.content_box().inset(-0.5);
                scene.stroke(rect, &Stroke::new(1.0), Color::BLACK).draw();
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
        (SQUARE_SIZE as u32 * 3, SQUARE_SIZE as u32 * 3),
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
                scene.fill(ctx.content_box(), Color::WHITE).draw();
                scene.fill(circle, RED).draw();
            })
            .post_paint_fn(move |_, _, _, painter| {
                let style = Stroke {
                    width: 4.0,
                    dash_pattern: Dashes::from_slice(&[12.0, 12.0]),
                    ..Default::default()
                };
                painter.stroke(circle, &style, Color::BLACK).draw();
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
        (SQUARE_SIZE as u32 * 2, SQUARE_SIZE as u32 * 2),
    );

    // The red circle should be clipped by the square.
    // The dashed circle shouldn't.
    assert_render_snapshot!(harness, "paint_clipping");
}

fn make_layer_split_tree(isolate_trailing_box: bool) -> NewWidget<impl Widget> {
    let leading = NewWidget::new(
        ModularWidget::new(())
            .measure_fn(|_, _, _, _, _, _| 20.)
            .paint_fn(|_, ctx, _, scene| {
                scene.fill(ctx.content_box(), RED).draw();
            }),
    );
    let trailing = NewWidget::new(
        ModularWidget::new(isolate_trailing_box)
            .measure_fn(|_, _, _, _, _, _| 20.)
            .paint_fn(|isolate, ctx, _, scene| {
                if *isolate {
                    ctx.set_paint_layer_mode(PaintLayerMode::IsolatedScene);
                }
                scene.fill(ctx.content_box(), BLUE).draw();
            }),
    );

    Flex::row()
        .with_fixed(leading)
        .with_fixed(trailing)
        .prepare()
}

fn make_external_placeholder_tree() -> NewWidget<impl Widget> {
    let leading = NewWidget::new(
        ModularWidget::new(())
            .measure_fn(|_, _, _, _, _, _| 20.)
            .paint_fn(|_, ctx, _, scene| {
                scene.fill(ctx.content_box(), RED).draw();
            }),
    );
    let placeholder = NewWidget::new(
        ModularWidget::new(())
            .measure_fn(|_, _, _, _, _, _| 20.)
            .layout_fn(|_, ctx, _, size| {
                ctx.set_clip_path(size.to_rect());
            })
            .paint_fn(|_, ctx, _, _scene| {
                ctx.set_paint_layer_mode(PaintLayerMode::External);
            }),
    );
    let trailing = NewWidget::new(
        ModularWidget::new(())
            .measure_fn(|_, _, _, _, _, _| 20.)
            .paint_fn(|_, ctx, _, scene| {
                scene.fill(ctx.content_box(), BLUE).draw();
            }),
    );

    Flex::row()
        .with_fixed(leading)
        .with_fixed(placeholder)
        .with_fixed(trailing)
        .prepare()
}

fn create_render_root(root_widget: NewWidget<impl Widget>) -> RenderRoot {
    let test_font = Blob::new(Arc::new(ROBOTO));
    RenderRoot::new(
        root_widget,
        |_| {},
        RenderRootOptions {
            default_properties: Arc::new(test_property_set()),
            use_system_fonts: false,
            size_policy: WindowSizePolicy::User,
            size: PhysicalSize::new(40, 20),
            scale_factor: 1.0,
            test_font: Some(test_font),
        },
    )
}

#[test]
fn isolated_scene_layers_update_the_plan_without_changing_rendering() {
    let mut inline_root = create_render_root(make_layer_split_tree(false));
    let (inline_layers, _) = inline_root.redraw();
    assert_eq!(inline_layers.layers.len(), 1);

    let mut isolated_root = create_render_root(make_layer_split_tree(true));
    let (isolated_layers, _) = isolated_root.redraw();
    assert_eq!(isolated_layers.layers.len(), 2);

    let mut inline_harness =
        TestHarness::create_with_size(test_property_set(), make_layer_split_tree(false), (40, 20));
    let mut isolated_harness =
        TestHarness::create_with_size(test_property_set(), make_layer_split_tree(true), (40, 20));

    assert_eq!(inline_harness.render(), isolated_harness.render());
}

#[test]
fn external_placeholders_appear_in_painter_order() {
    let mut root = create_render_root(make_external_placeholder_tree());
    let (visual_layers, _) = root.redraw();
    assert_eq!(visual_layers.layers.len(), 3);

    assert!(matches!(
        visual_layers.layers[0].kind,
        crate::app::VisualLayerKind::Scene(_)
    ));
    assert!(matches!(
        visual_layers.layers[1].kind,
        crate::app::VisualLayerKind::External { .. }
    ));
    assert!(matches!(
        visual_layers.layers[2].kind,
        crate::app::VisualLayerKind::Scene(_)
    ));
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

        let label = Label::new(text).prepare().with_props((
            Background::Color(Color::TRANSPARENT),
            Dimensions::fixed(20.px(), 20.px()),
        ));

        let mut props = PropertySet::new();
        if let Some(bg_color) = bg_color {
            props = props.with(Background::Color(bg_color));
        }

        Align::new(align, label).prepare().with_props(props)
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
    let grid_a = grid_a.prepare().with_props(props);
    let grid_b = grid_b.prepare().with_props(props);

    let mut root = ZStack::new();
    root = root.with(grid_a, ChildAlignment::ParentAligned);
    root = root.with(grid_b, ChildAlignment::ParentAligned);

    let mut harness = TestHarness::create_with_size(test_property_set(), root.prepare(), (350, 80));

    assert_render_snapshot!(harness, "paint_transparency");
}
