use crate::testing::{
    widget_ids, Harness, ModularWidget, Record, Recording, ReplaceChild, TestWidgetExt as _,
    REPLACE_CHILD,
};
use crate::widget::{Button, Flex, Label, SizedBox};
use crate::*;
use smallvec::smallvec;
use std::cell::Cell;
use std::collections::HashMap;
use std::rc::Rc;
use test_env_log::test;

#[test]
fn propagate_hot() {
    fn is_hot(harness: &Harness, id: WidgetId) -> bool {
        harness.get_widget(id).state().is_hot
    }

    fn next_hot_changed(recording: &Recording) -> Option<bool> {
        loop {
            let event = recording.next();
            if let Record::SC(StatusChange::HotChanged(hot)) = event {
                return Some(hot);
            }

            if let Record::None = event {
                return None;
            }
        }
    }

    let [button, pad, root, empty] = widget_ids();

    let root_rec = Recording::default();
    let padding_rec = Recording::default();
    let button_rec = Recording::default();

    let widget = Flex::column()
        .with_child_id(SizedBox::empty().width(10.0).height(10.0), empty)
        .with_child_id(
            Flex::column()
                .with_spacer(100.0)
                .with_child_id(Button::new("hot").record(&button_rec), button)
                .with_spacer(10.0)
                .record(&padding_rec),
            pad,
        )
        .record(&root_rec)
        .with_id(root);

    let mut harness = Harness::create(widget);

    // we don't care about setup events, so discard them now.
    root_rec.clear();
    padding_rec.clear();
    button_rec.clear();

    harness.inspect_widgets(|widget| assert!(!widget.state().is_hot));

    // What we are doing here is moving the mouse to different widgets,
    // and verifying both the widget's `is_hot` status and also that
    // each widget received the expected HotChanged messages.

    // Move to empty box

    harness.mouse_move_to(empty);

    assert!(is_hot(&harness, root));
    assert!(is_hot(&harness, empty));
    assert!(!is_hot(&harness, pad));

    assert_eq!(next_hot_changed(&root_rec), Some(true));
    assert_eq!(next_hot_changed(&padding_rec), None);
    assert_eq!(next_hot_changed(&button_rec), None);
    root_rec.clear();

    // Move to padding spacer of Flex column

    // Because mouse_move_to moves to the center of the widget, and the Flex::column
    // starts with a big spacer, the mouse is moved to the padding area, not the Button
    harness.mouse_move_to(pad);

    assert!(is_hot(&harness, pad));
    assert!(!is_hot(&harness, empty));
    assert!(!is_hot(&harness, button));
    assert!(is_hot(&harness, pad));

    assert_eq!(next_hot_changed(&root_rec), None);
    assert_eq!(next_hot_changed(&padding_rec), Some(true));
    assert_eq!(next_hot_changed(&button_rec), None);
    padding_rec.clear();

    // Move to button

    harness.mouse_move_to(button);

    assert!(is_hot(&harness, root));
    assert!(!is_hot(&harness, empty));
    assert!(is_hot(&harness, button));
    assert!(is_hot(&harness, pad));

    assert_eq!(next_hot_changed(&padding_rec), None);
    assert_eq!(next_hot_changed(&button_rec), Some(true));
    root_rec.clear();
    padding_rec.clear();
    button_rec.clear();

    // Move to empty box again

    harness.mouse_move_to(empty);

    assert!(is_hot(&harness, root));
    assert!(is_hot(&harness, empty));
    assert!(!is_hot(&harness, button));
    assert!(!is_hot(&harness, pad));

    assert_eq!(next_hot_changed(&root_rec), None);
    assert_eq!(next_hot_changed(&padding_rec), Some(false));
    assert_eq!(next_hot_changed(&button_rec), Some(false));
}
