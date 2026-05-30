// Copyright 2025 the Xilem Authors
// SPDX-License-Identifier: Apache-2.0

use assert_matches::assert_matches;

use crate::core::{ChildrenIds, NewWidget, Update, Widget, WidgetPod, WidgetTag};
use crate::kurbo::{Affine, Point, Rect, Size, Vec2};
use crate::layout::{AsUnit, Length, SizeDef};
use crate::testing::{ModularWidget, Record, TestHarness, TestWidgetExt};
use crate::tests::assert_rect_approx_eq;
use crate::theme::test_property_set;
use crate::widgets::SizedBox;

#[test]
fn request_compose() {
    struct ChildAndPos {
        child: WidgetPod<dyn Widget>,
        pos: Point,
        offset: Vec2,
    }

    let child_tag = WidgetTag::named("child");
    let parent_tag = WidgetTag::named("parent");
    let child = NewWidget::new(SizedBox::empty().record()).with_tag(child_tag);

    let child = ChildAndPos {
        child: child.erased().to_pod(),
        pos: Point::ZERO,
        offset: Vec2::ZERO,
    };

    let parent = ModularWidget::new(child)
        .measure_fn(|_state, _ctx, _props, _axis, _len_req, _cross_length| Length::ZERO)
        .layout_fn(|state, ctx, _props, size| {
            let child_size = ctx.compute_size(&mut state.child, SizeDef::fit(size), size.into());
            ctx.run_layout(&mut state.child, child_size);
            ctx.place_child(&mut state.child, state.pos);
        })
        .compose_fn(|state, ctx| {
            ctx.set_child_scroll_translation(&mut state.child, state.offset);
        })
        .register_children_fn(move |state, ctx| {
            ctx.register_child(&mut state.child);
        })
        .children_fn(|state| ChildrenIds::from_slice(&[state.child.id()]));
    let parent = NewWidget::new(parent.record()).with_tag(parent_tag);

    let mut harness = TestHarness::create(test_property_set(), parent);
    harness.flush_records_of(parent_tag);

    // Changing pos should lead to a layout and a compose pass.
    harness.edit_widget(parent_tag, |mut parent| {
        parent.widget.inner_mut().state.pos = Point::new(30., 30.);
        parent.ctx.request_layout();
    });
    assert_matches!(
        harness.take_records_of(parent_tag)[..],
        [Record::Layout(_), Record::Compose,]
    );

    // Changing scroll offset only should lead to a compose pass.
    harness.edit_widget(parent_tag, |mut parent| {
        parent.widget.inner_mut().state.offset = Vec2::new(8., 8.);
        parent.ctx.request_compose();
    });
    assert_matches!(harness.take_records_of(parent_tag)[..], [Record::Compose]);

    harness.edit_widget(parent_tag, |mut parent| {
        parent
            .ctx
            .set_transform(Affine::translate(Vec2::new(7., 7.)));
    });

    // Origin should be "parent_origin + pos + scroll_offset"
    let child = harness.get_widget(child_tag);
    let ctx = child.ctx();
    let origin = ctx.to_window(ctx.border_box().origin());
    assert_eq!(
        origin.to_vec2(),
        Vec2::new(7., 7.) + Point::new(30., 30.).to_vec2() + Vec2::new(8., 8.)
    );
}

#[test]
fn scroll_translation_updates_composed_geometry_without_layout() {
    struct ChildAndOffset {
        child: WidgetPod<dyn Widget>,
        offset: Vec2,
    }

    let child_tag = WidgetTag::unique();
    let parent_tag = WidgetTag::unique();
    let child = NewWidget::new(
        ModularWidget::new(())
            .measure_fn(|_, _, _, _, _, _| 10.3.px())
            .record(),
    )
    .with_tag(child_tag);

    let parent = ModularWidget::new(ChildAndOffset {
        child: child.erased().to_pod(),
        offset: Vec2::ZERO,
    })
    .layout_fn(|state, ctx, _, size| {
        let child_size = ctx.compute_size(&mut state.child, SizeDef::fit(size), size.into());
        ctx.run_layout(&mut state.child, child_size);
        ctx.place_child(&mut state.child, Point::new(5.1, 5.3));
    })
    .compose_fn(|state, ctx| {
        ctx.set_child_scroll_translation(&mut state.child, state.offset);
    })
    .register_children_fn(|state, ctx| {
        ctx.register_child(&mut state.child);
    })
    .children_fn(|state| ChildrenIds::from_slice(&[state.child.id()]))
    .prepare()
    .with_tag(parent_tag);

    let mut harness = TestHarness::create(test_property_set(), parent);
    harness.flush_records_of(child_tag);

    let hit_after_scroll = Point::new(16., 16.);
    harness.mouse_move(hit_after_scroll);
    let records = harness.take_records_of(child_tag);
    assert!(
        !records.iter().any(|record| matches!(
            record,
            Record::PointerEvent(_) | Record::Update(Update::HoveredChanged(true))
        )),
        "pointer should not reach the child before scroll translation"
    );
    assert!(!harness.get_widget(child_tag).ctx().is_hovered());

    harness.edit_widget(parent_tag, |mut parent| {
        parent.widget.state.offset = Vec2::new(2.2, 0.8);
        parent.ctx.request_compose();
    });

    let records = harness.take_records_of(child_tag);
    assert!(
        !records
            .iter()
            .any(|record| matches!(record, Record::Layout(_))),
        "scroll translation should not rerun child layout"
    );
    assert!(
        records.iter().any(|record| matches!(
            record,
            Record::PointerEvent(_) | Record::Update(Update::HoveredChanged(true))
        )),
        "stationary pointer should reach the child after scroll translation"
    );
    assert!(harness.get_widget(child_tag).ctx().is_hovered());

    let child = harness.get_widget(child_tag);
    let child_id = child.id();
    let ctx = child.ctx();
    assert_eq!(ctx.border_box().size(), Size::new(10., 11.));
    assert_rect_approx_eq(
        "window border box",
        ctx.window_transform().transform_rect_bbox(ctx.border_box()),
        Rect::new(7., 6., 17., 17.),
    );

    let _ = harness.redraw();
    let access_bounds = harness
        .access_node(child_id)
        .unwrap()
        .bounding_box()
        .unwrap();
    assert_rect_approx_eq(
        "access bounds",
        Rect::new(
            access_bounds.x0,
            access_bounds.y0,
            access_bounds.x1,
            access_bounds.y1,
        ),
        Rect::new(7., 6., 17., 17.),
    );
}

#[test]
fn scroll_pixel_snap() {
    let child_tag = WidgetTag::named("child");
    let child = NewWidget::new(SizedBox::empty()).with_tag(child_tag);

    let parent = ModularWidget::new_parent(child)
        .compose_fn(|state, ctx| {
            let offset = Vec2::new(0.1, 0.9);

            ctx.set_child_scroll_translation(state, offset);
        })
        .prepare();

    let harness = TestHarness::create(test_property_set(), parent);

    // Origin should be rounded to (0., 1.) by pixel-snapping.
    let child = harness.get_widget(child_tag);
    let ctx = child.ctx();
    let origin = ctx.to_window(ctx.border_box().origin());
    assert_eq!(origin, Point::new(0., 1.));
}
