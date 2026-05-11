// Copyright 2025 the Xilem Authors
// SPDX-License-Identifier: Apache-2.0

use assert_matches::assert_matches;

use crate::core::{ChildrenIds, NewWidget, Widget, WidgetPod, WidgetTag};
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
    let border_box = harness.get_widget(child_tag).ctx().border_box();
    let origin = harness
        .get_widget(child_tag)
        .ctx()
        .to_window(border_box.origin());
    assert_eq!(
        origin.to_vec2(),
        Vec2::new(7., 7.) + Point::new(30., 30.).to_vec2() + Vec2::new(8., 8.)
    );
}

#[test]
fn pixel_snapping() {
    let child_tag = WidgetTag::named("child");
    let child = NewWidget::new(SizedBox::empty().size(10.3.px(), 10.3.px())).with_tag(child_tag);
    let pos = Point::new(5.1, 5.3);
    let parent = ModularWidget::new_parent(child).layout_fn(move |child, ctx, _, size| {
        let child_size = ctx.compute_size(child, SizeDef::fit(size), size.into());
        ctx.run_layout(child, child_size);
        ctx.place_child(child, pos);
        ctx.set_baselines(2.4, 2.6);
    });
    let parent_tag = WidgetTag::named("parent");
    let parent = NewWidget::new(parent).with_tag(parent_tag);

    let harness = TestHarness::create(test_property_set(), parent);

    let child = harness.get_widget(child_tag);
    let ctx = child.ctx();
    let border_box = ctx.border_box();
    let content_box = ctx.content_box();
    let layout_content_box = ctx.layout_content_box();
    let child_pos = ctx.to_window(border_box.origin());
    let first_baseline = harness.get_widget(parent_tag).ctx().first_baseline();
    let last_baseline = harness.get_widget(parent_tag).ctx().last_baseline();

    assert_eq!(child_pos, Point::new(5.0, 5.0));
    assert_eq!(content_box.origin(), Point::ORIGIN);
    assert_rect_approx_eq(
        "layout_content_box",
        layout_content_box,
        Rect::from_origin_size(Point::new(0.1, 0.3), Size::new(10.3, 10.3)),
    );
    assert_eq!(border_box.size(), Size::new(10., 11.));
    assert_eq!(first_baseline, 2.4);
    assert_eq!(last_baseline, 2.6);
}

#[test]
fn pixel_snapping_after_window_transforms() {
    #[track_caller]
    fn assert_has_fractional_edge(name: &str, rect: Rect) {
        let edges = [rect.x0, rect.y0, rect.x1, rect.y1];
        assert!(
            edges.iter().any(|edge| (edge - edge.round()).abs() > 1e-9),
            "{name}: expected at least one fractional layout edge, got {rect:?}"
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
            ctx.run_layout(child, child_size);
            ctx.place_child(child, Point::new(1.7, 2.2));
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
            ctx.run_layout(child, child_size);
            ctx.place_child(child, Point::new(4.6, 3.9));
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
            ctx.run_layout(child, child_size);
            ctx.place_child(child, positions[idx]);
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

    let assert_snapped = |name: &str, tag: WidgetTag<SizedBox>| {
        let widget = harness.get_widget(tag);
        let ctx = widget.ctx();
        let layout_window = ctx
            .window_transform()
            .transform_rect_bbox(ctx.layout_border_box());
        let visual_window = ctx.window_transform().transform_rect_bbox(ctx.border_box());
        let expected_visual_window = Rect::new(
            layout_window.x0.round(),
            layout_window.y0.round(),
            layout_window.x1.round(),
            layout_window.y1.round(),
        );

        assert_has_fractional_edge(name, layout_window);
        assert_rect_approx_eq(name, visual_window, expected_visual_window);
    };

    assert_snapped("translated", translated_tag);
    assert_snapped("scaled", scaled_tag);
    assert_snapped("flipped", flipped_tag);
    assert_snapped("nested", nested_tag);
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

    let border_box = harness.get_widget(child_tag).ctx().border_box();
    let origin = harness
        .get_widget(child_tag)
        .ctx()
        .to_window(border_box.origin());

    // Origin should be rounded to (0., 1.) by pixel-snapping.
    assert_eq!(origin, Point::new(0., 1.));
}
