// Copyright 2025 the Xilem Authors
// SPDX-License-Identifier: Apache-2.0

use assert_matches::assert_matches;

use crate::core::{NewWidget, Widget, WidgetTag};
use crate::kurbo::{Affine, Insets, Point, Rect, Size, Vec2};
use crate::layout::{AsUnit, Length, SizeDef};
use crate::properties::{BorderWidth, Dimensions, Padding};
use crate::testing::{ModularWidget, TestHarness, TestWidgetExt, assert_debug_panics};
use crate::tests::{assert_point_approx_eq, assert_rect_approx_eq, assert_vec2_approx_eq};
use crate::theme::test_property_set;
use crate::widgets::{Button, ChildAlignment, Flex, Portal, SizedBox, ZStack};

#[test]
fn layout_simple() {
    const BOX_WIDTH: f64 = 50.;

    let tag_1 = WidgetTag::named("box1");
    let tag_2 = WidgetTag::named("box2");
    let box_side = Length::px(BOX_WIDTH);

    let widget = Flex::column()
        .with_fixed(
            Flex::row()
                .with_fixed(
                    NewWidget::new(SizedBox::empty().width(box_side).height(box_side))
                        .with_tag(tag_1),
                )
                .with_fixed(
                    NewWidget::new(SizedBox::empty().width(box_side).height(box_side))
                        .with_tag(tag_2),
                )
                .with_spacer(1.0)
                .prepare(),
        )
        .with_spacer(1.0)
        .prepare();

    let harness = TestHarness::create(test_property_set(), widget);

    let first_box_size = harness.get_widget(tag_1).ctx().layout_border_box().size();
    let first_box_paint_rect = harness.get_widget(tag_1).ctx().layout_paint_box();

    assert_eq!(first_box_size.width, BOX_WIDTH);
    assert_eq!(first_box_size.height, BOX_WIDTH);

    assert_eq!(first_box_paint_rect.x0, 0.0);
    assert_eq!(first_box_paint_rect.y0, 0.0);
    assert_eq!(first_box_paint_rect.x1, BOX_WIDTH);
    assert_eq!(first_box_paint_rect.y1, BOX_WIDTH);
}

#[test]
fn forget_to_recurse_layout() {
    let widget = ModularWidget::new_parent(Flex::row().prepare())
        .measure_fn(|_, _, _, _, _, _| Length::ZERO)
        .layout_fn(|_child, _ctx, _, _| {
            // We forget to call ctx.run_layout();
        })
        .prepare();

    assert_debug_panics!(
        TestHarness::create(test_property_set(), widget),
        "LayoutCtx::run_layout() was not called"
    );
}

#[test]
fn forget_to_call_place_child() {
    let widget = ModularWidget::new_parent(Flex::row().prepare())
        .layout_fn(|child, ctx, _, size| {
            // We call ctx.run_layout(), but forget place_child
            ctx.run_layout(child, size);
        })
        .prepare();

    assert_debug_panics!(
        TestHarness::create(test_property_set(), widget),
        "LayoutCtx::place_child() was not called"
    );
}

#[test]
fn call_place_child_before_layout() {
    let widget = ModularWidget::new_parent(Flex::row().prepare())
        .measure_fn(|_, _, _, _, _, _| Length::ZERO)
        .layout_fn(|child, ctx, _, _| {
            // We call ctx.place_child(), but forget run_layout
            ctx.place_child(child, Point::ORIGIN);
        })
        .prepare();

    assert_debug_panics!(
        TestHarness::create(test_property_set(), widget),
        "trying to call 'place_child'"
    );
}

#[test]
fn run_layout_on_stashed() {
    let parent_tag = WidgetTag::named("parent");
    let widget =
        ModularWidget::new_parent(Flex::row().prepare()).layout_fn(|child, ctx, _, size| {
            ctx.run_layout(child, size);
            ctx.place_child(child, Point::ZERO);
        });
    let widget = NewWidget::new(widget).with_tag(parent_tag);

    let mut harness = TestHarness::create(test_property_set(), widget);

    assert_debug_panics!(
        harness.edit_widget(parent_tag, |mut parent| {
            parent.ctx.set_stashed(&mut parent.widget.state, true);
            parent.ctx.request_layout();
        }),
        "trying to compute layout of stashed widget"
    );
}

#[test]
fn stash_then_run_layout() {
    let parent_tag = WidgetTag::named("parent");
    let widget =
        ModularWidget::new_parent(Flex::row().prepare()).layout_fn(|child, ctx, _, size| {
            // We check that stashing a widget is effective "immediately"
            // and triggers an error.
            ctx.set_stashed(child, true);
            ctx.run_layout(child, size);
            ctx.place_child(child, Point::ZERO);
        });
    let widget = NewWidget::new(widget).with_tag(parent_tag);

    assert_debug_panics!(
        TestHarness::create(test_property_set(), widget),
        "trying to compute layout of stashed widget"
    );
}

#[test]
fn unstash_then_run_layout() {
    let parent_tag = WidgetTag::named("parent");
    let widget =
        ModularWidget::new_parent(Flex::row().prepare()).layout_fn(|child, ctx, _, size| {
            // We check that unstashing a widget is effective "immediately"
            // and avoids an error.
            ctx.set_stashed(child, false);
            ctx.run_layout(child, size);
            ctx.place_child(child, Point::ZERO);
        });
    let widget = NewWidget::new(widget).with_tag(parent_tag);

    let mut harness = TestHarness::create(test_property_set(), widget);

    harness.edit_widget(parent_tag, |mut parent| {
        parent.ctx.set_stashed(&mut parent.widget.state, true);
        parent.ctx.request_layout();
    });
}

#[test]
fn skip_layout_when_cached() {
    let button_tag = WidgetTag::named("button");
    let sibling_tag = WidgetTag::named("sibling");

    let button = NewWidget::new(Button::with_text("Foobar").record()).with_tag(button_tag);
    let sibling =
        NewWidget::new(SizedBox::empty().width(20.px()).height(20.px())).with_tag(sibling_tag);

    // We choose a ZStack, because it should pass down the same constraints no matter what.
    let parent = NewWidget::new(
        ZStack::new()
            .with(button, ChildAlignment::ParentAligned)
            .with(sibling, ChildAlignment::ParentAligned),
    );

    let mut harness = TestHarness::create(test_property_set(), parent);

    harness.flush_records_of(button_tag);
    harness.edit_widget(sibling_tag, |mut sized_box| {
        SizedBox::set_width(&mut sized_box, 30.px());
        SizedBox::set_height(&mut sized_box, 30.px());
    });

    // The button did not request layout and its input constraints are the same:
    // Nothing should happen to it.
    let button_records_iter = harness.take_records_of(button_tag).into_iter();

    // Measurements will still happen with debug assertions enabled because we verify the cache.
    #[cfg(debug_assertions)]
    let button_records_iter =
        button_records_iter.filter(|r| !matches!(r, masonry_testing::Record::Measure(_)));

    let button_records: Vec<_> = button_records_iter.collect();
    assert_matches!(button_records[..], []);
}

#[test]
fn layout_insets() {
    const BOX_WIDTH: f64 = 50.;

    let child_tag = WidgetTag::named("child");
    let parent_tag = WidgetTag::named("parent");

    let child_widget = ModularWidget::new(())
        .measure_fn(|_, _, _, _, _, _| BOX_WIDTH.px())
        .layout_fn(|_, ctx, _, _| {
            // this widget paints twenty points above and below its layout bounds
            ctx.set_paint_insets(Insets::uniform_xy(0., 20.));
        });

    let parent_widget = NewWidget::new(SizedBox::new(
        NewWidget::new(child_widget).with_tag(child_tag),
    ))
    .with_tag(parent_tag);

    let root_widget = Portal::new(parent_widget).prepare();

    let harness = TestHarness::create(test_property_set(), root_widget);

    let child_paint_rect = harness.get_widget(child_tag).ctx().layout_paint_box();
    let parent_paint_rect = harness.get_widget(parent_tag).ctx().layout_paint_box();
    let parent_bounding_rect = harness.get_widget(parent_tag).ctx().bounding_box();

    // The child's paint box is affected by its paint insets
    assert_eq!(child_paint_rect.x0, 0.0);
    assert_eq!(child_paint_rect.y0, -20.0);
    assert_eq!(child_paint_rect.x1, BOX_WIDTH);
    assert_eq!(child_paint_rect.y1, BOX_WIDTH + 20.0);

    // The parent's paint box is not affected by the child's paint insets
    assert_eq!(parent_paint_rect.x0, 0.0);
    assert_eq!(parent_paint_rect.y0, 0.0);
    assert_eq!(parent_paint_rect.x1, BOX_WIDTH);
    assert_eq!(parent_paint_rect.y1, BOX_WIDTH);

    // The parent's bounding box is affected by the child's paint insets
    assert_eq!(parent_bounding_rect.x0, 0.0);
    assert_eq!(parent_bounding_rect.y0, -20.0);
    assert_eq!(parent_bounding_rect.x1, BOX_WIDTH);
    assert_eq!(parent_bounding_rect.y1, BOX_WIDTH + 20.0);
}

#[test]
fn content_box() {
    let tag = WidgetTag::named("hero");

    let props = (
        Dimensions::fixed(100.px(), 100.px()),
        Padding {
            left: 1.px(),
            right: 2.px(),
            top: 3.px(),
            bottom: 4.px(),
        },
        BorderWidth::all(1.px()),
    );

    let hero = NewWidget::new(Button::with_text("Hero"))
        .with_tag(tag)
        .with_props(props);

    let harness = TestHarness::create(test_property_set(), hero);

    let border_box = harness.get_widget(tag).ctx().layout_border_box();
    let border_box_size = border_box.size();
    let content_box = harness.get_widget(tag).ctx().layout_content_box();
    let content_box_size = content_box.size();
    let border_box_translation = harness.get_widget(tag).ctx().border_box_translation();

    let expected_border_box_size = Size::new(100., 100.);
    let expected_content_box_size = Size::new(95., 91.);

    assert_eq!(border_box_size, expected_border_box_size);
    assert_eq!(border_box.size(), expected_border_box_size);

    assert_eq!(content_box_size, expected_content_box_size);
    assert_eq!(content_box.size(), expected_content_box_size);

    let expected_border_box_origin = Point::new(-2., -4.);
    let expected_content_box_origin = Point::ORIGIN;

    assert_eq!(
        border_box_translation,
        -expected_border_box_origin.to_vec2()
    );

    let expected_border_box =
        Rect::from_origin_size(expected_border_box_origin, expected_border_box_size);
    let expected_content_box =
        Rect::from_origin_size(expected_content_box_origin, expected_content_box_size);

    assert_eq!(border_box, expected_border_box);
    assert_eq!(content_box, expected_content_box);
}

#[test]
fn boxes_match_without_insets_or_snapping() {
    let tag = WidgetTag::unique();
    let child = NewWidget::new(SizedBox::empty().size(10.px(), 8.px())).with_tag(tag);
    let root = ModularWidget::new_parent(child)
        .layout_fn(|child, ctx, _, size| {
            let child_size = ctx.compute_size(child, SizeDef::fit(size), size.into());
            ctx.run_layout(child, child_size);
            ctx.place_child(child, Point::new(2., 3.));
        })
        .prepare();

    let harness = TestHarness::create(test_property_set(), root);
    let child = harness.get_widget(tag);
    let ctx = child.ctx();

    // Everything besides the bounding box will be exactly the same
    let local_box = Rect::new(0., 0., 10., 8.);
    assert_rect_approx_eq("content_box", ctx.content_box(), local_box);
    assert_rect_approx_eq("layout_content_box", ctx.layout_content_box(), local_box);
    assert_rect_approx_eq("border_box", ctx.border_box(), local_box);
    assert_rect_approx_eq("layout_border_box", ctx.layout_border_box(), local_box);
    assert_rect_approx_eq("paint_box", ctx.paint_box(), local_box);
    assert_rect_approx_eq("layout_paint_box", ctx.layout_paint_box(), local_box);
    assert_rect_approx_eq(
        "bounding_box",
        ctx.bounding_box(),
        Rect::new(2., 3., 12., 11.),
    );
}

#[test]
fn boxes_use_visual_content_box_coordinates() {
    let tag = WidgetTag::unique();

    let child = ModularWidget::new(())
        .layout_fn(|_, ctx, _, _| {
            ctx.set_paint_insets(Insets::new(5.9, 6.1, 7.4, 8.2));
        })
        .prepare()
        .with_tag(tag)
        .with_props((
            Dimensions::fixed(23.4.px(), 17.6.px()),
            BorderWidth::all(0.7.px()),
            Padding {
                left: 1.2.px(),
                right: 2.3.px(),
                top: 3.4.px(),
                bottom: 4.5.px(),
            },
        ));

    let root = ModularWidget::new_parent(child)
        .layout_fn(|child, ctx, _, size| {
            let child_size = ctx.compute_size(child, SizeDef::fit(size), size.into());
            ctx.run_layout(child, child_size);
            ctx.place_child(child, Point::new(5.3, 7.6));
        })
        .prepare()
        .with_props(Dimensions::fixed(80.px(), 80.px()));

    let harness = TestHarness::create(test_property_set(), root);
    let child = harness.get_widget(tag);
    let ctx = child.ctx();

    // (5.3,7.6)..(28.7,25.2) snaps to (5,8)..(29,25), so visual origin is (-0.3,0.4).
    assert_vec2_approx_eq(
        "visual_translation",
        ctx.visual_translation(),
        Vec2::new(-0.3, 0.4),
    );
    // Border 0.7 + padding (1.2,3.4) gives top-left content inset (1.9,4.1).
    assert_vec2_approx_eq(
        "border_box_translation",
        ctx.border_box_translation(),
        Vec2::new(1.9, 4.1),
    );

    // Visual size (24,17) minus insets (1.9+3.0,4.1+5.2) gives content size (19.1,7.7).
    assert_rect_approx_eq(
        "content_box",
        ctx.content_box(),
        Rect::new(0., 0., 19.1, 7.7),
    );
    // Layout content (0,0)..(18.5,8.3) minus visual origin (-0.3,0.4) gives (0.3,-0.4)..(18.8,7.9).
    assert_rect_approx_eq(
        "layout_content_box",
        ctx.layout_content_box(),
        Rect::new(0.3, -0.4, 18.8, 7.9),
    );
    // Visual border box (-0.3,0.4)..(23.7,17.4) minus visual+border-box translation (1.6,4.5).
    assert_rect_approx_eq(
        "border_box",
        ctx.border_box(),
        Rect::new(-1.9, -4.1, 22.1, 12.9),
    );
    // Layout border box (0,0)..(23.4,17.6) minus visual+border-box translation (1.6,4.5).
    assert_rect_approx_eq(
        "layout_border_box",
        ctx.layout_border_box(),
        Rect::new(-1.6, -4.5, 21.8, 13.1),
    );
    // Paint insets become (4,2,4.4,3); visual border box + those,
    // minus visual+border-box translation (1.6,4.5).
    assert_rect_approx_eq(
        "paint_box",
        ctx.paint_box(),
        Rect::new(-5.9, -6.1, 26.5, 15.9),
    );
    // Layout border box + paint insets (4,2,4.4,3), minus visual+border-box translation (1.6,4.5).
    assert_rect_approx_eq(
        "layout_paint_box",
        ctx.layout_paint_box(),
        Rect::new(-5.6, -6.5, 26.2, 16.1),
    );
    // Visual paint box (-4.3,-1.6)..(28.1,20.4) plus layout origin (5.3,7.6).
    assert_rect_approx_eq(
        "bounding_box",
        ctx.bounding_box(),
        Rect::new(1., 6., 33.4, 28.),
    );

    // Local origin (== visual content box) maps to
    // visual+border-box translation (1.6,4.5) plus layout origin (5.3,7.6).
    assert_point_approx_eq(
        "to_window content box origin",
        ctx.to_window(Point::ORIGIN),
        Point::new(6.9, 12.1),
    );
    // Border box origin (-1.9,-4.1) plus visual+border-box translation (1.6,4.5)
    // gives visual origin (-0.3,0.4), then plus layout origin (5.3,7.6).
    assert_point_approx_eq(
        "to_window border box origin",
        ctx.to_window(ctx.border_box().origin()),
        Point::new(5., 8.),
    );
    // Inverse of content box window origin: (6.9,12.1) - (5.3,7.6) - (1.6,4.5) = (0,0).
    assert_point_approx_eq(
        "to_local content box origin",
        ctx.to_local(Point::new(6.9, 12.1)),
        Point::ORIGIN,
    );
    // window_transform adds visual+border-box translation (1.6,4.5) and layout origin (5.3,7.6).
    assert_point_approx_eq(
        "window_transform",
        ctx.window_transform() * Point::new(2., 1.),
        Point::new(8.9, 13.1),
    );
}

#[test]
fn visual_content_box_clamps_when_snapping_shrinks_below_insets() {
    let tag = WidgetTag::unique();

    let child = ModularWidget::new(()).prepare().with_tag(tag).with_props((
        Dimensions::fixed(2.5.px(), 2.5.px()),
        BorderWidth::all(0.5.px()),
        Padding {
            left: 0.7.px(),
            right: 0.8.px(),
            top: 0.6.px(),
            bottom: 0.9.px(),
        },
    ));

    let root = ModularWidget::new_parent(child)
        .layout_fn(|child, ctx, _, size| {
            let child_size = ctx.compute_size(child, SizeDef::fit(size), size.into());
            ctx.run_layout(child, child_size);
            ctx.place_child(child, Point::new(0.6, 0.6));
        })
        .prepare()
        .with_props(Dimensions::fixed(20.px(), 20.px()));

    let harness = TestHarness::create(test_property_set(), root);
    let child = harness.get_widget(tag);
    let ctx = child.ctx();

    // (0.6,0.6)..(3.1,3.1) snaps to (1,1)..(3,3), so visual origin is (0.4,0.4).
    assert_vec2_approx_eq(
        "visual_translation",
        ctx.visual_translation(),
        Vec2::new(0.4, 0.4),
    );
    // Border 0.5 + padding (0.7,0.6) gives top-left content inset (1.2,1.1).
    assert_vec2_approx_eq(
        "border_box_translation",
        ctx.border_box_translation(),
        Vec2::new(1.2, 1.1),
    );
    // Visual border box (0.4,0.4)..(2.4,2.4) minus visual+border-box translation (1.6,1.5).
    assert_rect_approx_eq(
        "border_box",
        ctx.border_box(),
        Rect::new(-1.2, -1.1, 0.8, 0.9),
    );
    // Visual size (2,2) is smaller than inset sums (1.2+1.3, 1.1+1.4), so content clamps to zero.
    assert_rect_approx_eq("content_box", ctx.content_box(), Rect::ZERO);
    // Layout content size is 2.5 - (1.2+1.3) = 0, then subtract visual origin (0.4,0.4).
    assert_rect_approx_eq(
        "layout_content_box",
        ctx.layout_content_box(),
        Rect::new(-0.4, -0.4, -0.4, -0.4),
    );
}

#[test]
fn transforms_handle_visual_content_box_space_translation() {
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
