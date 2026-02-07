// Copyright 2025 the Xilem Authors
// SPDX-License-Identifier: Apache-2.0

use assert_matches::assert_matches;

use crate::core::{NewWidget, Widget, WidgetOptions, WidgetTag};
use crate::kurbo::{Insets, Point, Rect, Size};
use crate::layout::{AsUnit, Length, SizeDef};
use crate::properties::{BorderWidth, Dimensions, Padding};
use crate::testing::{ModularWidget, TestHarness, TestWidgetExt, assert_debug_panics};
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
                .with_fixed(NewWidget::new_with_tag(
                    SizedBox::empty().width(box_side).height(box_side),
                    tag_1,
                ))
                .with_fixed(NewWidget::new_with_tag(
                    SizedBox::empty().width(box_side).height(box_side),
                    tag_2,
                ))
                .with_spacer(1.0)
                .with_auto_id(),
        )
        .with_spacer(1.0)
        .with_auto_id();

    let harness = TestHarness::create(test_property_set(), widget);

    let first_box_size = harness.get_widget(tag_1).ctx().border_box_size();
    let first_box_paint_rect = harness.get_widget(tag_1).ctx().paint_box();

    assert_eq!(first_box_size.width, BOX_WIDTH);
    assert_eq!(first_box_size.height, BOX_WIDTH);

    assert_eq!(first_box_paint_rect.x0, 0.0);
    assert_eq!(first_box_paint_rect.y0, 0.0);
    assert_eq!(first_box_paint_rect.x1, BOX_WIDTH);
    assert_eq!(first_box_paint_rect.y1, BOX_WIDTH);
}

#[test]
fn forget_to_recurse_layout() {
    let widget = ModularWidget::new_parent(Flex::row().with_auto_id())
        .measure_fn(|_, _, _, _, _, _| 0.)
        .layout_fn(|_child, _ctx, _, _| {
            // We forget to call ctx.run_layout();
        })
        .with_auto_id();

    assert_debug_panics!(
        TestHarness::create(test_property_set(), widget),
        "LayoutCtx::run_layout() was not called"
    );
}

#[test]
fn forget_to_call_place_child() {
    let widget = ModularWidget::new_parent(Flex::row().with_auto_id())
        .layout_fn(|child, ctx, _, size| {
            // We call ctx.run_layout(), but forget place_child
            ctx.run_layout(child, size);
        })
        .with_auto_id();

    assert_debug_panics!(
        TestHarness::create(test_property_set(), widget),
        "LayoutCtx::place_child() was not called"
    );
}

#[test]
fn call_place_child_before_layout() {
    let widget = ModularWidget::new_parent(Flex::row().with_auto_id())
        .measure_fn(|_, _, _, _, _, _| 0.)
        .layout_fn(|child, ctx, _, _| {
            // We call ctx.place_child(), but forget run_layout
            ctx.place_child(child, Point::ORIGIN);
        })
        .with_auto_id();

    assert_debug_panics!(
        TestHarness::create(test_property_set(), widget),
        "trying to call 'place_child'"
    );
}

#[test]
fn run_layout_on_stashed() {
    let parent_tag = WidgetTag::named("parent");
    let widget =
        ModularWidget::new_parent(Flex::row().with_auto_id()).layout_fn(|child, ctx, _, size| {
            ctx.run_layout(child, size);
            ctx.place_child(child, Point::ZERO);
        });
    let widget = NewWidget::new_with_tag(widget, parent_tag);

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
        ModularWidget::new_parent(Flex::row().with_auto_id()).layout_fn(|child, ctx, _, size| {
            // We check that stashing a widget is effective "immediately"
            // and triggers an error.
            ctx.set_stashed(child, true);
            ctx.run_layout(child, size);
            ctx.place_child(child, Point::ZERO);
        });
    let widget = NewWidget::new_with_tag(widget, parent_tag);

    assert_debug_panics!(
        TestHarness::create(test_property_set(), widget),
        "trying to compute layout of stashed widget"
    );
}

#[test]
fn unstash_then_run_layout() {
    let parent_tag = WidgetTag::named("parent");
    let widget =
        ModularWidget::new_parent(Flex::row().with_auto_id()).layout_fn(|child, ctx, _, size| {
            // We check that unstashing a widget is effective "immediately"
            // and avoids an error.
            ctx.set_stashed(child, false);
            ctx.run_layout(child, size);
            ctx.place_child(child, Point::ZERO);
        });
    let widget = NewWidget::new_with_tag(widget, parent_tag);

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

    let button = NewWidget::new_with_tag(Button::with_text("Foobar").record(), button_tag);
    let sibling = NewWidget::new_with_tag(
        SizedBox::empty().width(20.px()).height(20.px()),
        sibling_tag,
    );

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
fn pixel_snapping() {
    let child_tag = WidgetTag::named("child");
    let child = NewWidget::new_with_tag(SizedBox::empty().size(10.3.px(), 10.3.px()), child_tag);
    let pos = Point::new(5.1, 5.3);
    let parent = ModularWidget::new_parent(child).layout_fn(move |child, ctx, _, size| {
        let child_size = ctx.compute_size(child, SizeDef::fit(size), size.into());
        ctx.run_layout(child, child_size);
        ctx.place_child(child, pos);
        ctx.set_baseline_offset(5.1);
    });
    let parent_tag = WidgetTag::named("parent");
    let parent = NewWidget::new_with_tag(parent, parent_tag);

    let harness = TestHarness::create(test_property_set(), parent);

    let child_pos = harness.get_widget(child_tag).ctx().window_origin();
    let child_size = harness.get_widget(child_tag).ctx().border_box_size();
    let baseline = harness.get_widget(parent_tag).ctx().baseline_offset();

    assert_eq!(child_pos, Point::new(5.0, 5.0));
    assert_eq!(child_size, Size::new(10., 11.));
    assert_eq!(baseline, 5.);
}

#[test]
fn layout_insets() {
    const BOX_WIDTH: f64 = 50.;

    let child_tag = WidgetTag::named("child");
    let parent_tag = WidgetTag::named("parent");

    let child_widget = ModularWidget::new(())
        .measure_fn(|_, _, _, _, _, _| BOX_WIDTH)
        .layout_fn(|_, ctx, _, _| {
            // this widget paints twenty points above and below its layout bounds
            ctx.set_paint_insets(Insets::uniform_xy(0., 20.));
        });

    let parent_widget = NewWidget::new_with_tag(
        SizedBox::new(NewWidget::new_with_tag(child_widget, child_tag)),
        parent_tag,
    );

    let root_widget = Portal::new(parent_widget).with_auto_id();

    let harness = TestHarness::create(test_property_set(), root_widget);

    let child_paint_rect = harness.get_widget(child_tag).ctx().paint_box();
    let parent_paint_rect = harness.get_widget(parent_tag).ctx().paint_box();
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
            left: 1.,
            right: 2.,
            top: 3.,
            bottom: 4.,
        },
        BorderWidth::all(1.),
    );

    let hero = NewWidget::new_with(
        Button::with_text("Hero"),
        Some(tag),
        WidgetOptions::default(),
        props,
    );

    let harness = TestHarness::create(test_property_set(), hero);

    let border_box = harness.get_widget(tag).ctx().border_box();
    let border_box_size = harness.get_widget(tag).ctx().border_box_size();
    let content_box = harness.get_widget(tag).ctx().content_box();
    let content_box_size = harness.get_widget(tag).ctx().content_box_size();
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
