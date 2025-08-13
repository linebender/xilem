// Copyright 2025 the Xilem Authors
// SPDX-License-Identifier: Apache-2.0

use accesskit::NodeId;
use assert_matches::assert_matches;
use masonry_core::core::{NewWidget, Widget, WidgetTag};
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
    harness.flush_records_of(target_tag);
    harness.flush_records_of(parent_tag);

    harness.edit_widget(target_tag, |mut widget| {
        widget.ctx.request_accessibility_update();
    });
    let _ = harness.render();

    // Check that `Widget::accessibility()` is called for the child (which did request it)
    // but not the parent (which did not).
    let records = harness.take_records_of(target_tag);
    assert!(records.iter().any(|r| matches!(r, Record::Accessibility)));

    let records = harness.take_records_of(parent_tag);
    assert!(records.iter().all(|r| !matches!(r, Record::Accessibility)));

    // Check that `Widget::accessibility()` is not called: neither node has requested
    // an accessibility update.
    let _ = harness.render();
    assert_matches!(harness.take_records_of(target_tag)[..], []);
    assert_matches!(harness.take_records_of(parent_tag)[..], []);
}

#[test]
fn access_node_children() {
    let parent_tag = WidgetTag::new("parent");

    let child_1 = NewWidget::new(SizedBox::empty());
    let child_2 = NewWidget::new(SizedBox::empty());
    let child_3 = NewWidget::new(SizedBox::empty());

    let parent = NewWidget::new_with_tag(
        ModularWidget::new_multi_parent(vec![child_1, child_2, child_3]),
        parent_tag,
    );
    let grandparent = NewWidget::new(ModularWidget::new_parent(parent));

    let mut harness = TestHarness::create(default_property_set(), grandparent);
    let _ = harness.render();

    let parent_ref = harness.get_widget(parent_tag);
    let parent_node_id = parent_ref.id();
    let [id_1, id_2, id_3] = parent_ref.inner().children_ids()[..] else {
        unreachable!()
    };

    let parent_node = harness.access_node(parent_node_id).unwrap();
    assert_eq!(
        Vec::<NodeId>::from_iter(parent_node.child_ids()),
        vec![id_1, id_2, id_3]
    );

    // We stash a child
    harness.edit_widget(parent_tag, |mut parent| {
        parent.ctx.set_stashed(&mut parent.widget.state[1], true);
        parent.ctx.request_accessibility_update();
    });
    let _ = harness.render();

    // Stash child is not included
    let parent_node = harness.access_node(parent_node_id).unwrap();
    assert_eq!(
        Vec::<NodeId>::from_iter(parent_node.child_ids()),
        vec![id_1, id_3]
    );
}
