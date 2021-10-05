use crate::testing::{Harness, Record, Recording, ReplaceChild, TestWidgetExt as _, REPLACE_CHILD};
use crate::widget::{Flex, Label, SizedBox};
use crate::*;
use test_env_log::test;

#[test]
fn app_creation() {
    let record = Recording::default();
    let widget = SizedBox::empty().record(&record);

    let _harness = Harness::create(widget);

    assert!(matches!(record.next(), Record::L(LifeCycle::WidgetAdded)));
    assert!(matches!(
        record.next(),
        Record::L(LifeCycle::BuildFocusChain)
    ));
    assert!(matches!(record.next(), Record::E(Event::WindowConnected)));
    assert!(matches!(record.next(), Record::E(Event::WindowSize(_))));
    assert!(record.is_empty());
}

/// Test that lifecycle events are sent correctly to a child added during event
/// handling
#[test]
fn adding_child() {
    let record = Recording::default();
    let record_new_child = Recording::default();
    let record_new_child2 = record_new_child.clone();

    let replacer = ReplaceChild::new(Label::new(""), move || {
        Flex::row()
            .with_child(Label::new(""))
            .with_child(Label::new("").record(&record_new_child2))
    });

    let widget = Flex::row()
        .with_child(Label::new("hi").record(&record))
        .with_child(replacer);

    let mut harness = Harness::create(widget);
    record.clear();

    assert!(record_new_child.is_empty());

    harness.submit_command(REPLACE_CHILD);
    assert!(matches!(record.next(), Record::E(Event::Command(_))));

    assert!(matches!(
        dbg!(record_new_child.next()),
        Record::L(LifeCycle::WidgetAdded)
    ));
    assert!(matches!(
        record_new_child.next(),
        Record::L(LifeCycle::BuildFocusChain)
    ));
    assert!(record_new_child.is_empty());
}
