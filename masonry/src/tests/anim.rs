// Copyright 2025 the Xilem Authors
// SPDX-License-Identifier: Apache-2.0

use assert_matches::assert_matches;
use masonry_core::core::{NewWidget, WidgetTag};
use masonry_testing::{ModularWidget, Record, TestHarness, TestWidgetExt};

use crate::theme::default_property_set;
use crate::widgets::SizedBox;

#[test]
fn needs_anim_flag() {
    let target_tag = WidgetTag::new("target");
    let parent_tag = WidgetTag::new("parent");
    let child = NewWidget::new_with_tag(SizedBox::empty().record(), target_tag);
    let parent = NewWidget::new_with_tag(ModularWidget::new_parent(child).record(), parent_tag);
    let grandparent = NewWidget::new(ModularWidget::new_parent(parent));

    let mut harness = TestHarness::create(default_property_set(), grandparent);
    harness.flush_records_of(target_tag);
    harness.flush_records_of(parent_tag);

    harness.edit_widget_with_tag(target_tag, |mut widget| {
        widget.ctx.request_anim_frame();
    });
    harness.animate_ms(42);

    let records = harness.get_records_of(target_tag);
    assert!(
        records
            .iter()
            .any(|r| matches!(r, Record::AnimFrame(42_000_000)))
    );

    let records = harness.get_records_of(parent_tag);
    assert!(records.iter().all(|r| !matches!(r, Record::AnimFrame(_))));

    harness.animate_ms(42);

    // We didn't re-request an animation, so nothing should happen.
    assert_matches!(harness.get_records_of(parent_tag)[..], []);
}
