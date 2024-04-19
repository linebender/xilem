// This software is licensed under Apache License 2.0 and distributed on an
// "as-is" basis without warranties of any kind. See the LICENSE file for
// details.

use assert_matches::assert_matches;
use winit::event::MouseButton;

use crate::event::{PointerEvent, PointerState};
use crate::testing::{widget_ids, Record, Recording, TestHarness, TestWidgetExt as _};
use crate::widget::{Button, Flex, Label, SizedBox};
use crate::*;

fn is_hot(harness: &TestHarness, id: WidgetId) -> bool {
    harness.get_widget(id).state().is_hot
}

fn next_hot_changed(recording: &Recording) -> Option<bool> {
    while let Some(event) = recording.next() {
        match event {
            Record::SC(StatusChange::HotChanged(hot)) => return Some(hot),
            _ => {}
        }
    }
    None
}

#[test]
fn propagate_hot() {
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

    let mut harness = TestHarness::create(widget);

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

    dbg!(harness.get_widget(empty).state().layout_rect());

    eprintln!("root: {root:?}");
    eprintln!("empty: {empty:?}");
    eprintln!("pad: {pad:?}");
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

#[test]
fn update_hot_on_mouse_leave() {
    let [label_id] = widget_ids();

    let label_rec = Recording::default();

    let widget = Label::new("hello").with_id(label_id).record(&label_rec);

    let mut harness = TestHarness::create(widget);

    harness.mouse_move_to(label_id);
    assert!(is_hot(&harness, label_id));

    label_rec.clear();
    harness.process_pointer_event(PointerEvent::PointerLeave(PointerState::empty()));

    assert!(!is_hot(&harness, label_id));
    assert_eq!(next_hot_changed(&label_rec), Some(false));
}

// TODO - https://github.com/PoignardAzur/masonry-rs/issues/58
#[cfg(FALSE)]
#[test]
fn update_hot_from_layout() {
    pub const COLLAPSE: Selector = Selector::new("masonry-test.collapse");
    pub const BOX_SIZE: Size = Size::new(50.0, 50.0);

    let [collapsable_id, box_id] = widget_ids();

    let box_rec = Recording::default();

    let collapsable_box = ModularWidget::new(false)
        .event_fn(move |collapsed, ctx, event| {
            if let Event::Command(command) = event {
                if command.is(COLLAPSE) {
                    *collapsed = true;
                    ctx.request_layout();
                }
            }
        })
        .layout_fn(
            move |collapsed, _ctx, _bc| {
                if *collapsed {
                    Size::ZERO
                } else {
                    BOX_SIZE
                }
            },
        );

    let widget = Flex::row()
        .with_child(
            Flex::column()
                .with_child_id(collapsable_box, collapsable_id)
                .with_child_id(
                    SizedBox::empty().height(50.0).width(50.0).record(&box_rec),
                    box_id,
                )
                .with_flex_spacer(1.0),
        )
        .with_flex_spacer(1.0);

    let mut harness = TestHarness::create(widget);

    harness.mouse_move_to(collapsable_id);
    assert!(is_hot(&harness, collapsable_id));
    assert!(!is_hot(&harness, box_id));

    box_rec.clear();
    harness.submit_command(COLLAPSE);
    assert!(!is_hot(&harness, collapsable_id));
    assert!(is_hot(&harness, box_id));

    assert_eq!(next_hot_changed(&box_rec), Some(true));
}

#[test]
fn get_pointer_events_while_active() {
    fn next_pointer_event(recording: &Recording) -> Option<PointerEvent> {
        while let Some(event) = recording.next() {
            match event {
                Record::PE(event) => {
                    return Some(event);
                }
                _ => {}
            };
        }
        None
    }

    let [button, root, empty, empty_2] = widget_ids();

    let button_rec = Recording::default();

    let widget = Flex::column()
        .with_child_id(SizedBox::empty().width(10.0).height(10.0), empty)
        .with_child_id(SizedBox::empty().width(10.0).height(10.0), empty_2)
        .with_child_id(Button::new("hello").record(&button_rec), button)
        .with_id(root);

    let mut harness = TestHarness::create(widget);

    // First we check that the default state is clear: nothing active, no event recorded

    assert!(!harness.get_widget(button).state().is_active);
    assert!(!harness.get_widget(empty).state().is_active);
    assert!(!harness.get_widget(root).state().has_active);

    assert_matches!(next_pointer_event(&button_rec), None);

    // We press the button

    harness.mouse_move_to(button);
    harness.mouse_button_press(MouseButton::Left);

    assert_matches!(
        next_pointer_event(&button_rec),
        Some(PointerEvent::PointerMove(_))
    );
    assert_matches!(
        next_pointer_event(&button_rec),
        Some(PointerEvent::PointerDown(_, _))
    );
    assert_matches!(next_pointer_event(&button_rec), None);

    assert!(harness.get_widget(button).state().is_active);
    assert!(!harness.get_widget(empty).state().is_active);

    assert!(harness.get_widget(root).state().has_active);
    assert!(!harness.get_widget(root).state().is_active);

    // We move the cursor away without releasing the button

    harness.mouse_move_to(empty);

    assert_matches!(
        next_pointer_event(&button_rec),
        Some(PointerEvent::PointerMove(_))
    );
    assert_matches!(next_pointer_event(&button_rec), None);

    assert!(harness.get_widget(button).state().is_active);
    assert!(!harness.get_widget(empty).state().is_active);
    assert!(harness.get_widget(root).state().has_active);

    // We simulate the scroll wheel, still without releasing the button

    harness.mouse_wheel(Vec2::ZERO);

    assert_matches!(
        next_pointer_event(&button_rec),
        Some(PointerEvent::MouseWheel(_, _))
    );
    assert_matches!(next_pointer_event(&button_rec), None);

    // We release the button

    harness.mouse_button_release(MouseButton::Left);

    assert_matches!(
        next_pointer_event(&button_rec),
        Some(PointerEvent::PointerUp(_, _))
    );
    assert_matches!(next_pointer_event(&button_rec), None);

    assert!(!harness.get_widget(button).state().is_active);
    assert!(!harness.get_widget(empty).state().is_active);
    assert!(!harness.get_widget(root).state().has_active);

    // We move the mouse again to check movements aren't captured anymore
    harness.mouse_move_to(empty_2);
    assert_matches!(next_pointer_event(&button_rec), None);
}
