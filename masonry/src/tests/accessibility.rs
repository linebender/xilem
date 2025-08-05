// Copyright 2025 the Xilem Authors
// SPDX-License-Identifier: Apache-2.0

use assert_matches::assert_matches;
use masonry_core::core::{NewWidget, WidgetTag};
use masonry_testing::{ModularWidget, Record, TestHarness, TestWidgetExt};

use crate::theme::default_property_set;
use crate::widgets::SizedBox;

#[test]
fn request_accessibility() {
    let target_tag = WidgetTag::new("target");
    let parent_tag = WidgetTag::new("parent");
    let child = NewWidget::new_with_tag(SizedBox::empty().record(), target_tag);
    let parent = NewWidget::new_with_tag(ModularWidget::new_parent(child).record(), parent_tag);
    let grandparent = NewWidget::new(ModularWidget::new_parent(parent));

    let mut harness = TestHarness::create(default_property_set(), grandparent);
    let _ = harness.render();
    harness.flush_records_of(target_tag);
    harness.flush_records_of(parent_tag);

    harness.edit_widget_with_tag(target_tag, |mut widget| {
        widget.ctx.request_accessibility_update();
    });
    let _ = harness.render();

    // Check that `Widget::accessibility()` is called for the child (which did request it)
    // but not the parent (which did not).
    let records = harness.get_records_of(target_tag);
    assert!(records.iter().any(|r| matches!(r, Record::Accessibility)));

    let records = harness.get_records_of(parent_tag);
    assert!(records.iter().all(|r| !matches!(r, Record::Accessibility)));

    // Check that `Widget::accessibility()` is not called: neither node has requested
    // an accessibility update.
    let _ = harness.render();
    assert_matches!(harness.get_records_of(target_tag)[..], []);
    assert_matches!(harness.get_records_of(parent_tag)[..], []);
}
