use crate::testing::{
    widget_ids, Harness, ModularWidget, Record, Recording, ReplaceChild, TestWidgetExt as _,
    REPLACE_CHILD,
};
use crate::widget::{Flex, Label, SizedBox};
use crate::*;
use smallvec::smallvec;
use std::cell::Cell;
use std::collections::HashMap;
use std::rc::Rc;
use test_log::test;

/// Ensure that notifications are delivered to ancestors, but not siblings.
#[test]
fn notifications() {
    const NOTIFICATION: Selector = Selector::new("druid-tests.some-notification");

    let sender = ModularWidget::new(()).event_fn(|_, ctx, event, _| {
        if matches!(event, Event::WindowConnected) {
            ctx.init();
            ctx.submit_notification(NOTIFICATION);
        }
    });

    let sibling_rec = Recording::default();
    let parent_rec = Recording::default();
    let grandparent_rec = Recording::default();

    let tree = Flex::row()
        .with_child(sender)
        .with_child(SizedBox::empty().record(&sibling_rec))
        .record(&parent_rec)
        .record(&grandparent_rec);

    let saw_notification = |rec: &Recording| {
        rec.drain()
            .iter()
            .any(|ev| matches!(ev, Record::E(Event::Notification(_))))
    };

    let mut harness = Harness::create(tree);

    assert!(!saw_notification(&sibling_rec));
    assert!(saw_notification(&parent_rec));
    assert!(saw_notification(&grandparent_rec));
}
