// Copyright 2025 the Xilem Authors
// SPDX-License-Identifier: Apache-2.0

use assert_matches::assert_matches;
use masonry_core::core::{NewWidget, WidgetTag};
use masonry_testing::{ModularWidget, Record, TestHarness, TestWidgetExt};

use crate::theme::default_property_set;
use crate::widgets::SizedBox;

#[test]
fn request_paint() {
    let target_tag = WidgetTag::new("target");
    let parent_tag = WidgetTag::new("parent");
    let child = NewWidget::new_with_tag(SizedBox::empty().record(), target_tag);
    let parent = NewWidget::new_with_tag(ModularWidget::new_parent(child).record(), parent_tag);
    let grandparent = NewWidget::new(ModularWidget::new_parent(parent));

    let mut harness = TestHarness::create(default_property_set(), grandparent);
    let _ = harness.render();
    harness.flush_records_of(target_tag);
    harness.flush_records_of(parent_tag);

    // Paint
    harness.edit_widget_with_tag(target_tag, |mut widget| {
        widget.ctx.request_paint_only();
    });
    let _ = harness.render();

    // Check that `Widget::paint()` is called for the child (which did request it)
    // but not the parent (which did not).
    assert_matches!(harness.get_records_of(target_tag)[..], [Record::Paint]);
    assert_matches!(harness.get_records_of(parent_tag)[..], []);

    // Post-paint
    harness.edit_widget_with_tag(target_tag, |mut widget| {
        widget.ctx.request_post_paint();
    });
    let _ = harness.render();

    // Check that `Widget::post_paint()` is called for the child (which did request it)
    // but not the parent (which did not).
    assert_matches!(harness.get_records_of(target_tag)[..], [Record::PostPaint]);
    assert_matches!(harness.get_records_of(parent_tag)[..], []);

    // Check that `Widget::paint()` and `Widget::post_paint()` are not called:
    // neither widget has requested an update.
    let _ = harness.render();
    assert_matches!(harness.get_records_of(parent_tag)[..], []);
    assert_matches!(harness.get_records_of(target_tag)[..], []);
}

// TODO - Test painting order for widget with multiple children.

// TODO - Check that the widget's clip restricts what the widget paints.

// TODO - Check that `Widget::post_paint()` can paint outside the widget's clip.
