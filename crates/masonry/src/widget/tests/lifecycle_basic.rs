// This software is licensed under Apache License 2.0 and distributed on an
// "as-is" basis without warranties of any kind. See the LICENSE file for
// details.

#![allow(unused_imports)]

use insta::assert_debug_snapshot;

use crate::testing::{
    widget_ids, Record, Recording, ReplaceChild, TestHarness, TestWidgetExt as _,
};
use crate::widget::{Flex, Label, SizedBox};
use crate::*;

#[test]
fn app_creation() {
    let record = Recording::default();
    let widget = SizedBox::empty().record(&record);

    let _harness = TestHarness::create(widget);

    let record = record.drain();
    assert_debug_snapshot!(record);
}

// FIXME - Need to figure out this test
#[ignore]
#[cfg(FALSE)]
/// Test that lifecycle events are sent correctly to a child added during event
/// handling
#[test]
fn adding_child() {
    let record = Recording::default();
    let record_new_child = Recording::default();
    let record_new_child2 = record_new_child.clone();

    let replacer = ReplaceChild::new(Label::new(""), move || {
        Flex::row()
            .with_child(Label::new("hello"))
            .with_child(Label::new("world"))
            .with_child(Label::new("hi"))
            .with_child(Label::new("").record(&record_new_child2))
    });

    let widget = Flex::row()
        .with_child(Label::new("hi").record(&record))
        .with_child(replacer);

    let mut harness = TestHarness::create(widget);
    record.clear();

    assert!(record_new_child.is_empty());

    harness.submit_command(REPLACE_CHILD);
    assert!(matches!(record.next(), Record::E(Event::Command(_))));

    let record_new_child = record_new_child.drain();
    assert_debug_snapshot!(record_new_child);
}

#[test]
fn child_tracking() {
    let [id_1, id_2, id_3, id_4] = widget_ids();

    let widget = SizedBox::new_with_id(
        SizedBox::new_with_id(
            Flex::row()
                .with_child_id(SizedBox::empty(), id_1)
                .with_child_id(SizedBox::empty(), id_2),
            id_3,
        ),
        id_4,
    );

    let harness = TestHarness::create(widget);

    let root_state = harness.get_widget(id_4).state();
    assert_eq!(root_state.children.entry_count(), 3);
    assert!(root_state.children.may_contain(&id_1));
    assert!(root_state.children.may_contain(&id_2));
    assert!(root_state.children.may_contain(&id_3));

    let child_state = harness.get_widget(id_3).state();
    assert!(child_state.children.may_contain(&id_1));
    assert!(child_state.children.may_contain(&id_2));
    assert_eq!(child_state.children.entry_count(), 2);
}

/// Test that all children are registered correctly after a child is replaced.
#[test]
#[cfg(FALSE)]
fn register_after_adding_child() {
    let [id_1, id_2, id_3, id_4, id_5, id_6, id_8] = widget_ids();

    let replacer = ReplaceChild::new(Label::new("hello").with_id(id_1), move || {
        SizedBox::new_with_id(
            Flex::row()
                .with_child_id(SizedBox::empty(), id_2)
                .with_child_id(SizedBox::empty(), id_3),
            id_4,
        )
    });

    let widget = Flex::row()
        .with_child_id(Label::new("hi"), id_8)
        .with_child_id(replacer, id_6)
        .with_id(id_5);

    let mut harness = TestHarness::create(widget);

    let root_state = harness.get_widget(id_5).state();
    assert!(root_state.children.may_contain(&id_6));
    assert!(root_state.children.may_contain(&id_1));
    assert!(root_state.children.may_contain(&id_8));

    harness.submit_command(REPLACE_CHILD);

    let root_state = harness.get_widget(id_5).state();
    assert!(root_state.children.may_contain(&id_6));
    assert!(root_state.children.may_contain(&id_8));
    assert!(root_state.children.may_contain(&id_4));
    assert!(root_state.children.may_contain(&id_2));
    assert!(root_state.children.may_contain(&id_3));
}
