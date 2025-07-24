// Copyright 2020 the Xilem Authors and the Druid Authors
// SPDX-License-Identifier: Apache-2.0

//! Tests related to layout.

use crate::core::{NewWidget, Widget as _};
use crate::testing::{TestHarness, widget_ids};
use crate::theme::default_property_set;
use crate::widgets::{Flex, SizedBox};

#[test]
fn layout_simple() {
    const BOX_WIDTH: f64 = 50.;

    let [id_1, id_2] = widget_ids();

    let widget = Flex::column()
        .with_child(
            Flex::row()
                .with_child(NewWidget::new_with_id(
                    SizedBox::empty().width(BOX_WIDTH).height(BOX_WIDTH),
                    id_1,
                ))
                .with_child(NewWidget::new_with_id(
                    SizedBox::empty().width(BOX_WIDTH).height(BOX_WIDTH),
                    id_2,
                ))
                .with_flex_spacer(1.0)
                .with_auto_id(),
        )
        .with_flex_spacer(1.0);

    let harness = TestHarness::create(default_property_set(), widget);

    let first_box_rect = harness.get_widget(id_1).ctx().local_layout_rect();
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
