// Copyright 2025 the Xilem Authors
// SPDX-License-Identifier: Apache-2.0

use std::cell::Cell;
use std::rc::Rc;

use assert_matches::assert_matches;

use crate::core::{ChildrenIds, NewWidget, Widget, WidgetPod, WidgetTag, WindowEvent};
use crate::kurbo::{Affine, Insets, Point, Rect, Size, Vec2};
use crate::layout::{AsUnit, Length, SizeDef};
use crate::properties::{BorderWidth, Dimensions, Padding};
use crate::testing::{ModularWidget, Record, TestHarness, TestWidgetExt, assert_debug_panics};
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

    let first_box_size = harness.get_widget(tag_1).ctx().border_box().size();
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
    let widget = ModularWidget::new_parent(Flex::row().prepare())
        .measure_fn(|_, _, _, _, _, _| Length::ZERO)
        .layout_fn(|_child, _ctx, _, _| {
            // We forget to call ctx.layout_child();
        })
        .prepare();

    assert_debug_panics!(
        TestHarness::create(test_property_set(), widget),
        "LayoutCtx::layout_child() was not called"
    );
}

#[test]
fn call_child_size_before_layout() {
    let parent_tag = WidgetTag::unique();
    let layout_count = Rc::new(Cell::new(0));
    let layout_count_for_fn = layout_count.clone();

    let child = NewWidget::new(SizedBox::empty().width(20.px()).height(20.px()));
    let parent = ModularWidget::new_parent(child).layout_fn(move |child, ctx, _, size| {
        if layout_count_for_fn.get() > 0 {
            let _ = ctx.child_size(child);
        }

        layout_count_for_fn.set(layout_count_for_fn.get() + 1);

        let child_size = ctx.compute_size(child, SizeDef::fit(size), size.into());
        ctx.layout_child(child, Point::ORIGIN, child_size);
    });
    let widget = NewWidget::new(parent).with_tag(parent_tag);

    let mut harness = TestHarness::create(test_property_set(), widget);

    assert_debug_panics!(
        harness.edit_widget(parent_tag, |mut parent| {
            parent.ctx.request_layout();
        }),
        "trying to call 'child_size'"
    );
}

#[test]
fn layout_child_on_stashed() {
    let parent_tag = WidgetTag::named("parent");
    let widget =
        ModularWidget::new_parent(Flex::row().prepare()).layout_fn(|child, ctx, _, size| {
            ctx.layout_child(child, Point::ZERO, size);
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
fn stash_then_layout_child() {
    let parent_tag = WidgetTag::named("parent");
    let widget =
        ModularWidget::new_parent(Flex::row().prepare()).layout_fn(|child, ctx, _, size| {
            // We check that stashing a widget is effective "immediately"
            // and triggers an error.
            ctx.set_stashed(child, true);
            ctx.layout_child(child, Point::ZERO, size);
        });
    let widget = NewWidget::new(widget).with_tag(parent_tag);

    assert_debug_panics!(
        TestHarness::create(test_property_set(), widget),
        "trying to compute layout of stashed widget"
    );
}

#[test]
fn unstash_then_layout_child() {
    let parent_tag = WidgetTag::named("parent");
    let widget =
        ModularWidget::new_parent(Flex::row().prepare()).layout_fn(|child, ctx, _, size| {
            // We check that unstashing a widget is effective "immediately"
            // and avoids an error.
            ctx.set_stashed(child, false);
            ctx.layout_child(child, Point::ZERO, size);
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
    let button_records_iter = button_records_iter.filter(|r| !matches!(r, Record::Measure(_)));

    let button_records: Vec<_> = button_records_iter.collect();
    assert_matches!(button_records[..], []);
}

#[test]
fn pixel_snapping() {
    let child_tag = WidgetTag::unique();
    let child = ModularWidget::new(())
        .layout_fn(|_, _ctx, _, size| {
            assert_eq!(size, Size::new(10., 11.));
        })
        .prepare()
        .with_tag(child_tag)
        .with_props(Dimensions::fixed(10.3.px(), 10.3.px()));
    let pos = Point::new(5.1, 5.3);
    let parent = ModularWidget::new_parent(child).layout_fn(move |child, ctx, _, size| {
        let child_size = ctx.compute_size(child, SizeDef::fit(size), size.into());
        ctx.layout_child(child, pos, child_size);
        assert_eq!(ctx.child_size(child), Size::new(10., 11.));
        assert_point_approx_eq("child_origin", ctx.child_origin(child), Point::new(5., 5.));
        ctx.set_baselines(2.4, 2.6);
    });
    let parent_tag = WidgetTag::unique();
    let parent = NewWidget::new(parent).with_tag(parent_tag);

    let harness = TestHarness::create(test_property_set(), parent);

    let child = harness.get_widget(child_tag);
    let ctx = child.ctx();
    let border_box = ctx.border_box();
    let content_box = ctx.content_box();
    let child_pos = ctx.to_window(border_box.origin());
    let first_baseline = harness.get_widget(parent_tag).ctx().first_baseline();
    let last_baseline = harness.get_widget(parent_tag).ctx().last_baseline();

    assert_eq!(child_pos, Point::new(5.0, 5.0));
    assert_eq!(content_box.origin(), Point::ORIGIN);
    assert_eq!(content_box.size(), Size::new(10., 11.));
    assert_eq!(border_box.size(), Size::new(10., 11.));
    assert_eq!(first_baseline, 2.4);
    assert_eq!(last_baseline, 2.6);
}

#[test]
fn equal_nested_sizes_snap_consistently() {
    let parent_tag = WidgetTag::unique();
    let child_tag = WidgetTag::unique();

    let child = NewWidget::new(SizedBox::empty()).with_tag(child_tag);
    let parent = ModularWidget::new_parent(child)
        .layout_fn(|child, ctx, _, size| {
            ctx.layout_child(child, Point::ORIGIN, size);
        })
        .prepare();
    let parent = parent
        .with_tag(parent_tag)
        .with_props(Dimensions::fixed(70.6.px(), 10.2.px()));

    let root = ModularWidget::new_parent(parent)
        .layout_fn(|child, ctx, _, size| {
            let child_size = ctx.compute_size(child, SizeDef::fit(size), size.into());
            ctx.layout_child(child, Point::new(84.7, 0.0), child_size);
        })
        .prepare();

    let harness = TestHarness::create(test_property_set(), root);
    let parent = harness.get_widget(parent_tag);
    let child = harness.get_widget(child_tag);
    let parent_ctx = parent.ctx();
    let child_ctx = child.ctx();
    let parent_rect = parent_ctx
        .window_transform()
        .transform_rect_bbox(parent_ctx.border_box());
    let child_rect = child_ctx
        .window_transform()
        .transform_rect_bbox(child_ctx.border_box());

    assert_eq!(
        parent_ctx.border_box().size(),
        child_ctx.border_box().size()
    );
    assert_rect_approx_eq("parent_rect", parent_rect, Rect::new(85., 0., 155., 10.));
    assert_rect_approx_eq("child_rect", child_rect, Rect::new(85., 0., 155., 10.));
}

#[test]
fn shared_edges_snap_without_gaps_or_overlaps() {
    let tag_1 = WidgetTag::unique();
    let tag_2 = WidgetTag::unique();
    let tag_3 = WidgetTag::unique();

    let child_1 = NewWidget::new(SizedBox::empty()).with_tag(tag_1).erased();
    let child_2 = NewWidget::new(SizedBox::empty()).with_tag(tag_2).erased();
    let child_3 = NewWidget::new(SizedBox::empty()).with_tag(tag_3).erased();

    let root = ModularWidget::new(vec![child_1.to_pod(), child_2.to_pod(), child_3.to_pod()])
        .layout_fn(|children, ctx, _, _| {
            let placements = [
                (Point::new(0.0, 0.0), Size::new(33.3, 10.0)),
                (Point::new(33.3, 0.0), Size::new(33.3, 10.0)),
                (Point::new(66.6, 0.0), Size::new(33.4, 10.0)),
            ];
            for (child, (origin, size)) in children.iter_mut().zip(placements) {
                ctx.layout_child(child, origin, size);
            }
        })
        .register_children_fn(|children, ctx| {
            for child in children {
                ctx.register_child(child);
            }
        })
        .children_fn(|children| children.iter().map(|child| child.id()).collect())
        .prepare();

    let harness = TestHarness::create(test_property_set(), root);
    let rect = |tag: WidgetTag<SizedBox>| {
        let widget = harness.get_widget(tag);
        let ctx = widget.ctx();
        ctx.window_transform().transform_rect_bbox(ctx.border_box())
    };
    let rect_1 = rect(tag_1);
    let rect_2 = rect(tag_2);
    let rect_3 = rect(tag_3);

    assert_rect_approx_eq("rect_1", rect_1, Rect::new(0., 0., 33., 10.));
    assert_rect_approx_eq("rect_2", rect_2, Rect::new(33., 0., 67., 10.));
    assert_rect_approx_eq("rect_3", rect_3, Rect::new(67., 0., 100., 10.));
    assert_eq!(rect_1.x1, rect_2.x0);
    assert_eq!(rect_2.x1, rect_3.x0);
}

#[test]
fn rescale_requests_layout_for_pixel_snapping() {
    let child_tag = WidgetTag::unique();
    let child =
        NewWidget::new(SizedBox::empty().size(10.3.px(), 10.3.px()).record()).with_tag(child_tag);
    let parent = ModularWidget::new_parent(child)
        .layout_fn(|child, ctx, _, size| {
            let child_size = ctx.compute_size(child, SizeDef::fit(size), size.into());
            ctx.layout_child(child, Point::new(5.1, 5.3), child_size);
        })
        .prepare();

    let mut harness = TestHarness::create(test_property_set(), parent);

    assert_eq!(
        harness.get_widget(child_tag).ctx().border_box().size(),
        Size::new(10., 11.)
    );

    harness.flush_records_of(child_tag);
    harness.process_window_event(WindowEvent::Rescale(2.0));

    assert!(
        harness
            .take_records_of(child_tag)
            .into_iter()
            .any(|record| matches!(record, Record::Layout(_))),
        "scale factor changes must rerun layout because layout sizes can change under pixel snapping"
    );
    assert_eq!(
        harness.get_widget(child_tag).ctx().border_box().size(),
        Size::new(10.5, 10.)
    );
}

#[test]
fn pixel_snapping_can_be_disabled() {
    let parent_tag = WidgetTag::unique();
    let child_tag = WidgetTag::unique();
    let child = NewWidget::new(SizedBox::empty().size(10.3.px(), 10.3.px())).with_tag(child_tag);
    let pos = Point::new(5.1, 5.3);
    let parent = ModularWidget::new_parent(child).layout_fn(move |child, ctx, _, size| {
        let child_size = ctx.compute_size(child, SizeDef::fit(size), size.into());
        ctx.layout_child(child, pos, child_size);
    });
    let parent = NewWidget::new(parent)
        .with_snap_disabled(true)
        .with_tag(parent_tag);

    let mut harness = TestHarness::create(test_property_set(), parent);

    let child = harness.get_widget(child_tag);
    let ctx = child.ctx();
    let border_box = ctx.border_box();
    let child_pos = ctx.to_window(border_box.origin());

    assert_rect_approx_eq(
        "border_box",
        border_box,
        Rect::from_origin_size(Point::ORIGIN, Size::new(10.3, 10.3)),
    );
    assert_eq!(child_pos, pos);

    harness.edit_widget(parent_tag, |mut parent| {
        parent.set_snap_disabled(false);
    });

    let child = harness.get_widget(child_tag);
    let ctx = child.ctx();
    let border_box = ctx.border_box();
    let child_pos = ctx.to_window(border_box.origin());

    assert_eq!(child_pos, Point::new(5., 5.));
    assert_eq!(border_box.size(), Size::new(10., 11.));
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

    let border_box = harness.get_widget(tag).ctx().border_box();
    let border_box_size = border_box.size();
    let content_box = harness.get_widget(tag).ctx().content_box();
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
            ctx.layout_child(child, Point::new(2., 3.), child_size);
        })
        .prepare();

    let harness = TestHarness::create(test_property_set(), root);
    let child = harness.get_widget(tag);
    let ctx = child.ctx();

    // Everything besides the bounding box will be exactly the same
    let local_box = Rect::new(0., 0., 10., 8.);
    assert_rect_approx_eq("content_box", ctx.content_box(), local_box);
    assert_rect_approx_eq("border_box", ctx.border_box(), local_box);
    assert_rect_approx_eq("paint_box", ctx.paint_box(), local_box);
    assert_rect_approx_eq(
        "bounding_box",
        ctx.bounding_box(),
        Rect::new(2., 3., 12., 11.),
    );
}

#[test]
fn boxes_use_content_box_coordinates() {
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
            ctx.layout_child(child, Point::new(5.3, 7.6), child_size);
        })
        .prepare()
        .with_props(Dimensions::fixed(80.px(), 80.px()));

    let harness = TestHarness::create(test_property_set(), root);
    let child = harness.get_widget(tag);
    let ctx = child.ctx();

    // Border 0.7 + padding (1.2,3.4) gives top-left content inset (1.9,4.1).
    // The chosen child origin is (5.3,7.6), and snapping rounds its border-box to (5,8)-(29,25).
    assert_vec2_approx_eq(
        "border_box_translation",
        ctx.border_box_translation(),
        Vec2::new(1.9, 4.1),
    );

    assert_rect_approx_eq(
        "content_box",
        ctx.content_box(),
        Rect::new(0., 0., 19.1, 7.7),
    );
    assert_rect_approx_eq(
        "border_box",
        ctx.border_box(),
        Rect::new(-1.9, -4.1, 22.1, 12.9),
    );
    assert_rect_approx_eq(
        "paint_box",
        ctx.paint_box(),
        Rect::new(-5.9, -6.1, 26.5, 15.9),
    );
    assert_rect_approx_eq(
        "bounding_box",
        ctx.bounding_box(),
        Rect::new(1., 6., 33.4, 28.),
    );

    assert_point_approx_eq(
        "to_window content box origin",
        ctx.to_window(Point::ORIGIN),
        Point::new(6.9, 12.1),
    );
    assert_point_approx_eq(
        "to_window border box origin",
        ctx.to_window(ctx.border_box().origin()),
        Point::new(5., 8.),
    );
    assert_point_approx_eq(
        "to_local content box origin",
        ctx.to_local(Point::new(6.9, 12.1)),
        Point::ORIGIN,
    );
    assert_point_approx_eq(
        "window_transform",
        ctx.window_transform() * Point::new(2., 1.),
        Point::new(8.9, 13.1),
    );
}

#[test]
fn resnap_layout_when_intermediate_size_is_unchanged() {
    let root_tag = WidgetTag::unique();
    let leaf_tag = WidgetTag::unique();

    let leaf =
        NewWidget::new(SizedBox::empty().size(10.3.px(), 10.3.px()).record()).with_tag(leaf_tag);

    let middle = ModularWidget::new_parent(leaf)
        .layout_fn(|child, ctx, _, size| {
            let child_size = ctx.compute_size(child, SizeDef::fit(size), size.into());
            ctx.layout_child(child, Point::ORIGIN, child_size);
        })
        .prepare()
        .with_props(Dimensions::fixed(20.px(), 20.px()));

    let root = ModularWidget::new_parent(middle)
        .layout_fn(|child, ctx, _, size| {
            let child_size = ctx.compute_size(child, SizeDef::fit(size), size.into());
            ctx.layout_child(child, Point::ORIGIN, child_size);
        })
        .prepare()
        .with_tag(root_tag);

    let mut harness = TestHarness::create(test_property_set(), root);

    assert_eq!(
        harness.get_widget(leaf_tag).ctx().border_box().size(),
        Size::new(10., 10.)
    );

    harness.flush_records_of(leaf_tag);
    harness.edit_widget(root_tag, |mut root| {
        root.set_transform(Affine::scale(2.));
    });

    assert!(
        harness
            .take_records_of(leaf_tag)
            .into_iter()
            .any(|record| matches!(record, Record::Layout(_))),
        "leaf layout should rerun because snapping depends on inherited scale"
    );
    assert_eq!(
        harness.get_widget(leaf_tag).ctx().border_box().size(),
        Size::new(10.5, 10.5)
    );
}

#[test]
fn cached_layout_origin_changes_update_window_transforms() {
    struct MovingRoot {
        child: WidgetPod<dyn Widget>,
        layout_origin: Point,
        move_origin: Option<Point>,
    }

    fn assert_no_layout(records: Vec<Record>, name: &str) {
        assert!(
            !records
                .into_iter()
                .any(|record| matches!(record, Record::Layout(_))),
            "{name} should have reused its cached layout"
        );
    }

    let root_tag = WidgetTag::unique();
    let middle_tag = WidgetTag::unique();
    let leaf_tag = WidgetTag::unique();

    let leaf = NewWidget::new(SizedBox::empty().size(5.px(), 6.px()).record()).with_tag(leaf_tag);

    let middle = ModularWidget::new_parent(leaf).layout_fn(|child, ctx, _, size| {
        let child_size = ctx.compute_size(child, SizeDef::fit(size), size.into());
        ctx.layout_child(child, Point::new(3., 4.), child_size);
    });
    let middle = NewWidget::new(middle.record())
        .with_tag(middle_tag)
        .with_props(Dimensions::fixed(20.px(), 20.px()));

    let root_state = MovingRoot {
        child: middle.erased().to_pod(),
        layout_origin: Point::ORIGIN,
        move_origin: None,
    };
    let root = ModularWidget::new(root_state)
        .register_children_fn(|state, ctx| {
            ctx.register_child(&mut state.child);
        })
        .layout_fn(|state, ctx, _, size| {
            let child_size = ctx.compute_size(&mut state.child, SizeDef::fit(size), size.into());
            ctx.layout_child(&mut state.child, state.layout_origin, child_size);
            if let Some(move_origin) = state.move_origin {
                ctx.move_child(&mut state.child, move_origin);
            }
        })
        .children_fn(|state| ChildrenIds::from_slice(&[state.child.id()]))
        .prepare()
        .with_tag(root_tag);

    let mut harness = TestHarness::create(test_property_set(), root);

    assert_point_approx_eq(
        "initial middle window origin",
        harness.get_widget(middle_tag).ctx().window_transform() * Point::ORIGIN,
        Point::ORIGIN,
    );
    assert_point_approx_eq(
        "initial leaf window origin",
        harness.get_widget(leaf_tag).ctx().window_transform() * Point::ORIGIN,
        Point::new(3., 4.),
    );

    harness.flush_records_of(middle_tag);
    harness.flush_records_of(leaf_tag);
    harness.edit_widget(root_tag, |mut root| {
        root.widget.state.layout_origin = Point::ORIGIN;
        root.widget.state.move_origin = None;
        root.ctx.request_layout();
    });
    assert_no_layout(harness.take_records_of(middle_tag), "middle");
    assert_no_layout(harness.take_records_of(leaf_tag), "leaf");

    harness.edit_widget(root_tag, |mut root| {
        root.widget.state.layout_origin = Point::new(10., 20.);
        root.widget.state.move_origin = None;
        root.ctx.request_layout();
    });
    assert_no_layout(harness.take_records_of(middle_tag), "middle");
    assert_no_layout(harness.take_records_of(leaf_tag), "leaf");
    assert_point_approx_eq(
        "moved middle window origin",
        harness.get_widget(middle_tag).ctx().window_transform() * Point::ORIGIN,
        Point::new(10., 20.),
    );
    assert_point_approx_eq(
        "moved leaf window origin",
        harness.get_widget(leaf_tag).ctx().window_transform() * Point::ORIGIN,
        Point::new(13., 24.),
    );

    harness.edit_widget(root_tag, |mut root| {
        root.widget.state.layout_origin = Point::new(10., 20.);
        root.widget.state.move_origin = Some(Point::new(30., 40.));
        root.ctx.request_layout();
    });
    assert_no_layout(harness.take_records_of(middle_tag), "middle");
    assert_no_layout(harness.take_records_of(leaf_tag), "leaf");
    assert_point_approx_eq(
        "move_child middle window origin",
        harness.get_widget(middle_tag).ctx().window_transform() * Point::ORIGIN,
        Point::new(30., 40.),
    );
    assert_point_approx_eq(
        "move_child leaf window origin",
        harness.get_widget(leaf_tag).ctx().window_transform() * Point::ORIGIN,
        Point::new(33., 44.),
    );
}

#[test]
fn move_child_respects_snap_state() {
    fn check(snap_disabled: bool, expected_size: Size, expected_origin: Point) {
        let child = ModularWidget::new(())
            .prepare()
            .with_snap_disabled(snap_disabled)
            .with_props(Dimensions::fixed(10.3.px(), 10.3.px()));

        let root = ModularWidget::new_parent(child)
            .layout_fn(move |child, ctx, _, size| {
                let child_size = ctx.compute_size(child, SizeDef::fit(size), size.into());
                ctx.layout_child(child, Point::new(5.1, 5.3), child_size);
                ctx.move_child(child, Point::new(6.4, 7.6));

                assert_eq!(ctx.child_size(child), expected_size);
                assert_point_approx_eq("child_origin", ctx.child_origin(child), expected_origin);
            })
            .prepare()
            .with_props(Dimensions::fixed(80.px(), 80.px()));

        let _harness = TestHarness::create(test_property_set(), root);
    }

    check(false, Size::new(10., 11.), Point::new(6., 8.));
    check(true, Size::new(10.3, 10.3), Point::new(6.4, 7.6));
}

#[test]
fn move_child_quantizes_delta_in_window_space() {
    let child = ModularWidget::new(())
        .prepare()
        .with_props(Dimensions::fixed(10.px(), 10.px()));

    let root = ModularWidget::new_parent(child)
        .layout_fn(|child, ctx, _, size| {
            let child_size = ctx.compute_size(child, SizeDef::fit(size), size.into());
            ctx.layout_child(child, Point::ORIGIN, child_size);
            ctx.move_child(child, Point::new(0.3, 1.2));

            assert_point_approx_eq("child_origin", ctx.child_origin(child), Point::new(0.5, 2.));
        })
        .prepare()
        .with_transform(Affine::scale_non_uniform(-2., 0.5))
        .with_props(Dimensions::fixed(80.px(), 80.px()));

    let _harness = TestHarness::create(test_property_set(), root);
}

#[test]
fn content_box_clamps_when_insets_exceed_size() {
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
            ctx.layout_child(child, Point::new(0.6, 0.6), child_size);
        })
        .prepare()
        .with_props(Dimensions::fixed(20.px(), 20.px()));

    let harness = TestHarness::create(test_property_set(), root);
    let child = harness.get_widget(tag);
    let ctx = child.ctx();

    // Border 0.5 + padding (0.7,0.6) gives top-left content inset (1.2,1.1).
    assert_vec2_approx_eq(
        "border_box_translation",
        ctx.border_box_translation(),
        Vec2::new(1.2, 1.1),
    );
    assert_rect_approx_eq(
        "border_box",
        ctx.border_box(),
        Rect::new(-1.2, -1.1, 0.8, 0.9),
    );
    assert_rect_approx_eq("content_box", ctx.content_box(), Rect::ZERO);
}
