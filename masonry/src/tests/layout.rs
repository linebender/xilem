// Copyright 2025 the Xilem Authors
// SPDX-License-Identifier: Apache-2.0

use assert_matches::assert_matches;
use masonry_testing::{Record, TestWidgetExt, assert_debug_panics};

use crate::core::Widget;
use crate::core::{NewWidget, WidgetTag};
use crate::kurbo::{Insets, Point, Size};
use crate::layout::{AsUnit, LayoutSize, Length, SizeDef};
use crate::testing::{ModularWidget, TestHarness};
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

    let first_box_rect = harness.get_widget(tag_1).ctx().local_layout_rect();
    let first_box_paint_rect = harness.get_widget(tag_1).ctx().paint_rect();

    assert_eq!(first_box_rect.x0, 0.0);
    assert_eq!(first_box_rect.y0, 0.0);
    assert_eq!(first_box_rect.x1, BOX_WIDTH);
    assert_eq!(first_box_rect.y1, BOX_WIDTH);

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
    // Nothing besides measurements should happen to it.
    let button_records: Vec<_> = harness
        .take_records_of(button_tag)
        .into_iter()
        .filter(|r| !matches!(r, Record::Measure(_)))
        .collect();
    assert_matches!(button_records[..], []);
}

#[test]
fn pixel_snapping() {
    let child_tag = WidgetTag::named("child");
    let child = NewWidget::new_with_tag(SizedBox::empty().size(10.3.px(), 10.3.px()), child_tag);
    let pos = Point::new(5.1, 5.3);
    let parent = ModularWidget::new_parent(child)
        .measure_fn(|child, ctx, _props, axis, len_req, cross_length| {
            let auto_size = SizeDef::req(axis, len_req);
            let context_size = LayoutSize::maybe(axis.cross(), cross_length);

            ctx.compute_length(child, auto_size, context_size, axis, cross_length)
        })
        .layout_fn(move |child, ctx, _, size| {
            let child_size = ctx.compute_size(child, SizeDef::fit(size), size.into());
            ctx.run_layout(child, child_size);
            ctx.place_child(child, pos);
        });
    let parent = NewWidget::new(parent);

    let harness = TestHarness::create(test_property_set(), parent);

    let child_pos = harness.get_widget(child_tag).ctx().window_origin();
    let child_size = harness.get_widget(child_tag).ctx().size();

    assert_eq!(child_pos, Point::new(5.0, 5.0));
    assert_eq!(child_size, Size::new(10., 11.));
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

    let child_paint_rect = harness.get_widget(child_tag).ctx().paint_rect();
    let parent_paint_rect = harness.get_widget(parent_tag).ctx().paint_rect();

    assert_eq!(child_paint_rect.x0, 0.0);
    assert_eq!(child_paint_rect.y0, -20.0);
    assert_eq!(child_paint_rect.x1, BOX_WIDTH);
    assert_eq!(child_paint_rect.y1, BOX_WIDTH + 20.0);

    assert_eq!(parent_paint_rect.x0, 0.0);
    assert_eq!(parent_paint_rect.y0, -20.0);
    assert_eq!(parent_paint_rect.x1, BOX_WIDTH);
    assert_eq!(parent_paint_rect.y1, BOX_WIDTH + 20.0);
}
