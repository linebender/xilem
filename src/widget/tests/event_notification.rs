// This software is licensed under Apache License 2.0 and distributed on an
// "as-is" basis without warranties of any kind. See the LICENSE file for
// details.

use crate::testing::{ModularWidget, Record, Recording, TestHarness, TestWidgetExt as _};
use crate::widget::{Flex, SizedBox};
use crate::*;

/// Ensure that notifications are delivered to ancestors, but not siblings.
#[test]
fn notifications() {
    const NOTIFICATION: Selector = Selector::new("masonry-test.some-notification");

    let sender = ModularWidget::new(()).event_fn(|_, ctx, event| {
        if matches!(event, Event::WindowConnected) {
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

    let _harness = TestHarness::create(tree);

    assert!(!saw_notification(&sibling_rec));
    assert!(saw_notification(&parent_rec));
    assert!(saw_notification(&grandparent_rec));
}
