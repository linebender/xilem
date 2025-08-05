// Copyright 2025 the Xilem Authors
// SPDX-License-Identifier: Apache-2.0

use assert_matches::assert_matches;
use masonry_core::core::{ChildrenIds, NewWidget, Widget, WidgetPod, WidgetTag};
use masonry_testing::{ModularWidget, Record, TestHarness, TestWidgetExt};
use vello::kurbo::{Affine, Point, Size, Vec2};

use crate::theme::default_property_set;
use crate::widgets::SizedBox;

#[test]
fn request_compose() {
    struct ChildAndPos {
        child: WidgetPod<dyn Widget>,
        pos: Point,
        offset: Vec2,
    }

    let child_tag = WidgetTag::new("child");
    let parent_tag = WidgetTag::new("parent");
    let child = NewWidget::new_with_tag(SizedBox::empty().record(), child_tag);

    let child = ChildAndPos {
        child: child.erased().to_pod(),
        pos: Point::ZERO,
        offset: Vec2::ZERO,
    };

    let parent = ModularWidget::new(child)
        .layout_fn(|state, ctx, _props, bc| {
            ctx.run_layout(&mut state.child, bc);
            ctx.place_child(&mut state.child, state.pos);
            Size::ZERO
        })
        .compose_fn(|state, ctx| {
            ctx.set_child_scroll_translation(&mut state.child, state.offset);
        })
        .register_children_fn(move |state, ctx| {
            ctx.register_child(&mut state.child);
        })
        .children_fn(|state| ChildrenIds::from_slice(&[state.child.id()]));
    let parent = NewWidget::new_with_tag(parent.record(), parent_tag);

    let mut harness = TestHarness::create(default_property_set(), parent);
    harness.flush_records_of(parent_tag);

    // Changing pos should lead to a layout and a compose pass.
    harness.edit_widget_with_tag(parent_tag, |mut parent| {
        parent.widget.inner_mut().state.pos = Point::new(30., 30.);
        parent.ctx.request_layout();
    });
    assert_matches!(
        harness.get_records_of(parent_tag)[..],
        [Record::Layout(_), Record::Compose,]
    );

    // Changing scroll offset only should lead to a compose pass.
    harness.edit_widget_with_tag(parent_tag, |mut parent| {
        parent.widget.inner_mut().state.offset = Vec2::new(8., 8.);
        parent.ctx.request_compose();
    });
    assert_matches!(harness.get_records_of(parent_tag)[..], [Record::Compose]);

    harness.edit_widget_with_tag(parent_tag, |mut parent| {
        parent
            .ctx
            .set_transform(Affine::translate(Vec2::new(7., 7.)));
    });

    // Origin should be "parent_origin + pos + scroll_offset"
    let origin = harness.get_widget_with_tag(child_tag).ctx().window_origin();
    assert_eq!(
        origin.to_vec2(),
        Vec2::new(7., 7.) + Point::new(30., 30.).to_vec2() + Vec2::new(8., 8.)
    );
}
