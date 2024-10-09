// Copyright 2020 the Xilem Authors and the Druid Authors
// SPDX-License-Identifier: Apache-2.0

//! Tests related to layout.

use vello::kurbo::{Insets, Size};

use crate::testing::{widget_ids, ModularWidget, TestHarness, TestWidgetExt};
use crate::widget::{Flex, SizedBox};

#[test]
fn layout_simple() {
    const BOX_WIDTH: f64 = 50.;

    let [id_1, id_2] = widget_ids();

    let widget = Flex::column()
        .with_child(
            Flex::row()
                .with_child_id(SizedBox::empty().width(BOX_WIDTH).height(BOX_WIDTH), id_1)
                .with_child_id(SizedBox::empty().width(BOX_WIDTH).height(BOX_WIDTH), id_2)
                .with_flex_spacer(1.0),
        )
        .with_flex_spacer(1.0);

    let harness = TestHarness::create(widget);

    let first_box_rect = harness.get_widget(id_1).ctx().layout_rect();
    let first_box_paint_rect = harness.get_widget(id_1).ctx().paint_rect();

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
fn layout_insets() {
    const BOX_WIDTH: f64 = 50.;

    let [child_id, parent_id] = widget_ids();

    let child_widget = ModularWidget::new(()).layout_fn(|_, ctx, _| {
        // this widget paints twenty points above below its layout bounds
        ctx.set_paint_insets(Insets::uniform_xy(0., 20.));
        Size::new(BOX_WIDTH, BOX_WIDTH)
    });

    let parent_widget = SizedBox::new_with_id(child_widget, child_id).with_id(parent_id);

    let harness = TestHarness::create(parent_widget);

    let child_paint_rect = harness.get_widget(child_id).ctx().paint_rect();
    let parent_paint_rect = harness.get_widget(parent_id).ctx().paint_rect();

    assert_eq!(child_paint_rect.x0, 0.0);
    assert_eq!(child_paint_rect.y0, -20.0);
    assert_eq!(child_paint_rect.x1, BOX_WIDTH);
    assert_eq!(child_paint_rect.y1, BOX_WIDTH + 20.0);

    assert_eq!(parent_paint_rect.x0, 0.0);
    assert_eq!(parent_paint_rect.y0, -20.0);
    assert_eq!(parent_paint_rect.x1, BOX_WIDTH);
    assert_eq!(parent_paint_rect.y1, BOX_WIDTH + 20.0);
}

// TODO - insets + flex
// TODO - viewport
// TODO - insets + viewport
