#![allow(unused_imports)]

use super::*;

#[cfg(FALSE)]
#[test]
fn propagate_hot() {
    let [button, pad, root, empty] = widget_ids();

    let root_rec = Recording::default();
    let padding_rec = Recording::default();
    let button_rec = Recording::default();

    let widget = Split::columns(
        SizedBox::empty().with_id(empty),
        Button::new("hot")
            .record(&button_rec)
            .with_id(button)
            .padding(50.)
            .record(&padding_rec)
            .with_id(pad),
    )
    .record(&root_rec)
    .with_id(root);

    #[allow(clippy::cognitive_complexity)]
    Harness::create_simple((), widget, |harness| {
        harness.send_initial_events();
        harness.just_layout();

        // we don't care about setup events, so discard them now.
        root_rec.clear();
        padding_rec.clear();
        button_rec.clear();

        harness.inspect_state(|state| assert!(!state.is_hot));

        // What we are doing here is moving the mouse to different widgets,
        // and verifying both the widget's `is_hot` status and also that
        // each widget received the expected HotChanged messages.

        harness.event(Event::MouseMove(move_mouse((10., 10.))));
        assert!(harness.get_state(root).is_hot);
        assert!(harness.get_state(empty).is_hot);
        assert!(!harness.get_state(pad).is_hot);

        assert!(matches!(
            root_rec.next(),
            Record::L(LifeCycle::HotChanged(true))
        ));
        assert!(matches!(root_rec.next(), Record::E(Event::MouseMove(_))));
        assert!(root_rec.is_empty() && padding_rec.is_empty() && button_rec.is_empty());

        harness.event(Event::MouseMove(move_mouse((210., 10.))));

        assert!(harness.get_state(root).is_hot);
        assert!(!harness.get_state(empty).is_hot);
        assert!(!harness.get_state(button).is_hot);
        assert!(harness.get_state(pad).is_hot);

        assert!(matches!(root_rec.next(), Record::E(Event::MouseMove(_))));
        assert!(matches!(
            padding_rec.next(),
            Record::L(LifeCycle::HotChanged(true))
        ));
        assert!(matches!(padding_rec.next(), Record::E(Event::MouseMove(_))));
        assert!(root_rec.is_empty() && padding_rec.is_empty() && button_rec.is_empty());

        harness.event(Event::MouseMove(move_mouse((260., 60.))));
        assert!(harness.get_state(root).is_hot);
        assert!(!harness.get_state(empty).is_hot);
        assert!(harness.get_state(button).is_hot);
        assert!(harness.get_state(pad).is_hot);

        assert!(matches!(root_rec.next(), Record::E(Event::MouseMove(_))));
        assert!(matches!(padding_rec.next(), Record::E(Event::MouseMove(_))));
        assert!(matches!(
            button_rec.next(),
            Record::L(LifeCycle::HotChanged(true))
        ));
        assert!(matches!(button_rec.next(), Record::E(Event::MouseMove(_))));
        assert!(root_rec.is_empty() && padding_rec.is_empty() && button_rec.is_empty());

        harness.event(Event::MouseMove(move_mouse((10., 10.))));
        assert!(harness.get_state(root).is_hot);
        assert!(harness.get_state(empty).is_hot);
        assert!(!harness.get_state(button).is_hot);
        assert!(!harness.get_state(pad).is_hot);

        assert!(matches!(root_rec.next(), Record::E(Event::MouseMove(_))));
        assert!(matches!(
            padding_rec.next(),
            Record::L(LifeCycle::HotChanged(false))
        ));
        assert!(matches!(padding_rec.next(), Record::E(Event::MouseMove(_))));
        assert!(matches!(
            button_rec.next(),
            Record::L(LifeCycle::HotChanged(false))
        ));
        assert!(matches!(button_rec.next(), Record::E(Event::MouseMove(_))));
        assert!(root_rec.is_empty() && padding_rec.is_empty() && button_rec.is_empty());
    });
}

/// Ensure that notifications are delivered to ancestors, but not siblings.
#[cfg(FALSE)]
#[test]
fn notifications() {
    const NOTIFICATION: Selector = Selector::new("druid-tests.some-notification");

    let sender = ModularWidget::new(()).event_fn(|_, ctx, event, _, _| {
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
        .padding(10.0)
        .record(&grandparent_rec);

    let saw_notification = |rec: &Recording| {
        rec.drain()
            .any(|ev| matches!(ev, Record::E(Event::Notification(_))))
    };
    Harness::create_simple((), tree, |harness| {
        harness.send_initial_events();
        assert!(!saw_notification(&sibling_rec));
        assert!(saw_notification(&parent_rec));
        assert!(saw_notification(&grandparent_rec));
    });
}
