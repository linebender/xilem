use crate::testing::{Harness, ModularWidget, Record, Recording, TestWidgetExt as _};
use crate::widget::{Flex, SizedBox};
use crate::*;


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

    let _harness = Harness::create(tree);

    assert!(!saw_notification(&sibling_rec));
    assert!(saw_notification(&parent_rec));
    assert!(saw_notification(&grandparent_rec));
}
