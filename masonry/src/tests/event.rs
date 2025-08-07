// Copyright 2025 the Xilem Authors
// SPDX-License-Identifier: Apache-2.0

use accesskit::ActionRequest;
use assert_matches::assert_matches;
use masonry_core::core::{AccessEvent, NewWidget, TextEvent, Widget, WidgetTag};
use masonry_testing::{ModularWidget, Record, TestHarness, TestWidgetExt};
use ui_events::keyboard::{Key, NamedKey};
use ui_events::pointer::{PointerButton, PointerEvent, PointerInfo, PointerType};
use vello::kurbo::Size;

use crate::theme::default_property_set;
use crate::widgets::{Button, Flex, SizedBox, TextArea};

// POINTER EVENTS

fn create_capture_target() -> ModularWidget<()> {
    ModularWidget::new(())
        .pointer_event_fn(|_, ctx, _, event| {
            if matches!(event, PointerEvent::Down { .. }) {
                ctx.capture_pointer();
            }
        })
        .layout_fn(|_, _, _, _| Size::new(10., 10.))
}

#[test]
fn pointer_event() {
    let button_tag = WidgetTag::new("button");

    let button = NewWidget::new_with_tag(Button::with_text("button").record(), button_tag);

    let mut harness = TestHarness::create(default_property_set(), button);
    let button_id = harness.get_widget_with_tag(button_tag).id();

    harness.flush_records_of(button_tag);
    harness.mouse_move_to(button_id);

    let records = harness.get_records_of(button_tag);
    assert!(
        records
            .iter()
            .any(|r| matches!(r, Record::PointerEvent(PointerEvent::Move(_))))
    );
}

#[test]
fn pointer_event_bubbling() {
    let button_tag = WidgetTag::new("button");
    let parent_tag = WidgetTag::new("parent");
    let grandparent_tag = WidgetTag::new("grandparent");

    let button = NewWidget::new_with_tag(Button::with_text("button").record(), button_tag);
    let parent = NewWidget::new_with_tag(ModularWidget::new_parent(button).record(), parent_tag);
    let grandparent =
        NewWidget::new_with_tag(ModularWidget::new_parent(parent).record(), grandparent_tag);

    let mut harness = TestHarness::create(default_property_set(), grandparent);
    let button_id = harness.get_widget_with_tag(button_tag).id();

    harness.flush_records_of(button_tag);
    harness.mouse_click_on(button_id);

    let has_pointer_down = |records: Vec<_>| {
        records
            .iter()
            .any(|r| matches!(r, Record::PointerEvent(PointerEvent::Down { .. })))
    };

    assert!(has_pointer_down(harness.get_records_of(button_tag)));
    assert!(has_pointer_down(harness.get_records_of(parent_tag)));
    assert!(has_pointer_down(harness.get_records_of(grandparent_tag)));
}

#[test]
fn pointer_capture_and_cancel() {
    let target_tag = WidgetTag::new("target");

    let target = create_capture_target();
    let target = NewWidget::new_with_tag(target, target_tag);

    let mut harness = TestHarness::create(default_property_set(), target);

    let target_id = harness.get_widget_with_tag(target_tag).id();

    harness.mouse_move_to(target_id);
    harness.mouse_button_press(PointerButton::Primary);
    assert_eq!(harness.pointer_capture_target_id(), Some(target_id));

    harness.process_pointer_event(PointerEvent::Cancel(PointerInfo {
        pointer_id: None,
        persistent_device_id: None,
        pointer_type: PointerType::default(),
    }));
    assert_eq!(harness.pointer_capture_target_id(), None);
}

#[test]
fn pointer_capture_suppresses_neighbors() {
    let target_tag = WidgetTag::new("target");
    let other_tag = WidgetTag::new("other");

    let target = create_capture_target();
    let target = NewWidget::new_with_tag(target, target_tag);

    let other = SizedBox::empty().width(10.).height(10.);
    let other = NewWidget::new_with_tag(other.record(), other_tag);

    let parent = Flex::column()
        .with_child(target)
        .with_child(other)
        .with_auto_id();

    let mut harness = TestHarness::create(default_property_set(), parent);
    harness.flush_records_of(other_tag);

    let target_id = harness.get_widget_with_tag(target_tag).id();
    let other_id = harness.get_widget_with_tag(other_tag).id();

    harness.mouse_move_to(target_id);
    harness.mouse_button_press(PointerButton::Primary);

    assert_eq!(harness.pointer_capture_target_id(), Some(target_id));

    // As long as 'target' is captured, 'other' doesn't get pointer events, even when the cursor is on it.
    harness.mouse_move_to(other_id);
    assert_matches!(harness.get_records_of(other_tag)[..], []);

    // 'other' is not considered hovered either.
    assert!(!harness.get_widget_with_tag(other_tag).ctx().is_hovered());

    // We end pointer capture.
    harness.mouse_button_release(PointerButton::Primary);
    assert_eq!(harness.pointer_capture_target_id(), None);

    // Once the capture is released, 'other' should immediately register as hovered.
    assert!(harness.get_widget_with_tag(other_tag).ctx().is_hovered());
}

#[test]
fn pointer_cancel_on_window_blur() {
    let target_tag = WidgetTag::new("target");

    let target = create_capture_target();
    let target = NewWidget::new_with_tag(target.record(), target_tag);

    let mut harness = TestHarness::create(default_property_set(), target);

    let target_id = harness.get_widget_with_tag(target_tag).id();

    harness.mouse_move_to(target_id);
    harness.mouse_button_press(PointerButton::Primary);
    assert_eq!(harness.pointer_capture_target_id(), Some(target_id));
    harness.flush_records_of(target_tag);

    harness.process_text_event(TextEvent::WindowFocusChange(false));

    let records = harness.get_records_of(target_tag);
    assert!(
        records
            .iter()
            .any(|r| matches!(r, Record::PointerEvent(PointerEvent::Cancel(..))))
    );
}

#[test]
fn click_anchors_focus() {
    let child_3 = WidgetTag::new("child_3");
    let child_4 = WidgetTag::new("child_4");
    let other = WidgetTag::new("other");

    let parent = Flex::column()
        .with_child(NewWidget::new_with_tag(
            SizedBox::empty().width(5.).height(5.),
            other,
        ))
        .with_child(NewWidget::new(Button::with_text("")))
        .with_child(NewWidget::new(Button::with_text("")))
        .with_child(NewWidget::new_with_tag(Button::with_text(""), child_3))
        .with_child(NewWidget::new_with_tag(Button::with_text(""), child_4))
        .with_child(NewWidget::new(Button::with_text("")))
        .with_auto_id();

    let mut harness = TestHarness::create(default_property_set(), parent);

    let child_3_id = harness.get_widget_with_tag(child_3).id();
    let child_4_id = harness.get_widget_with_tag(child_4).id();
    let other_id = harness.get_widget_with_tag(other).id();

    // Clicking a button doesn't focus it.
    harness.mouse_click_on(child_3_id);
    assert_eq!(harness.focused_widget_id(), None);

    // But the next tab event focuses its neighbor.
    harness.process_text_event(TextEvent::key_down(Key::Named(NamedKey::Tab)));
    assert_eq!(harness.focused_widget_id(), Some(child_4_id));

    // Clicking another non-focusable widget clears focus.
    harness.mouse_click_on(other_id);
    assert_eq!(harness.focused_widget_id(), None);
}

// TEXT EVENTS

#[test]
fn text_event() {
    let target_tag = WidgetTag::new("target");

    let target = NewWidget::new_with_tag(TextArea::new_editable("").record(), target_tag);

    let mut harness = TestHarness::create(default_property_set(), target);
    let target_id = harness.get_widget_with_tag(target_tag).id();
    harness.flush_records_of(target_tag);

    // The widget isn't focused, it doesn't get text events.
    harness.keyboard_type_chars("A");
    assert_matches!(harness.get_records_of(target_tag)[..], []);

    // We focus on the widget, now it gets text events.
    harness.focus_on(Some(target_id));
    harness.keyboard_type_chars("A");
    let records = harness.get_records_of(target_tag);
    assert!(records.iter().any(|r| matches!(r, Record::TextEvent(_))));
}

#[test]
fn text_event_bubbling() {
    let target_tag = WidgetTag::new("target");
    let parent_tag = WidgetTag::new("parent");
    let grandparent_tag = WidgetTag::new("grandparent");

    let target = NewWidget::new_with_tag(
        ModularWidget::new(()).accepts_focus(true).record(),
        target_tag,
    );
    let parent = NewWidget::new_with_tag(ModularWidget::new_parent(target).record(), parent_tag);
    let grandparent =
        NewWidget::new_with_tag(ModularWidget::new_parent(parent).record(), grandparent_tag);

    let mut harness = TestHarness::create(default_property_set(), grandparent);
    let target_id = harness.get_widget_with_tag(target_tag).id();

    harness.focus_on(Some(target_id));
    harness.process_text_event(TextEvent::key_down(Key::Character("A".into())));

    let has_keyboard_event = |records: Vec<_>| {
        records
            .iter()
            .any(|r| matches!(r, Record::TextEvent(TextEvent::Keyboard(_))))
    };

    assert!(has_keyboard_event(harness.get_records_of(target_tag)));
    assert!(has_keyboard_event(harness.get_records_of(parent_tag)));
    assert!(has_keyboard_event(harness.get_records_of(grandparent_tag)));
}

#[test]
fn text_event_fallback() {
    let target_tag = WidgetTag::new("target");
    let parent_tag = WidgetTag::new("parent");

    let child = NewWidget::new_with_tag(TextArea::new_editable("").record(), target_tag);
    let parent = NewWidget::new_with_tag(Flex::row().with_child(child), parent_tag);

    let mut harness = TestHarness::create(default_property_set(), parent);
    harness.flush_records_of(target_tag);

    // If the root widget has exactly one child, that child gets text events when no widget is focused.
    harness.keyboard_type_chars("A");
    let records = harness.get_records_of(target_tag);
    assert!(records.iter().any(|r| matches!(r, Record::TextEvent(_))));

    // Unless it's disabled.
    harness.edit_widget_with_tag(target_tag, |mut target| {
        target.ctx.set_disabled(true);
    });
    harness.flush_records_of(target_tag);
    harness.keyboard_type_chars("A");
    assert_matches!(harness.get_records_of(target_tag)[..], []);

    harness.edit_widget_with_tag(target_tag, |mut target| {
        target.ctx.set_disabled(false);
    });
    harness.edit_widget_with_tag(parent_tag, |mut flex| {
        Flex::add_child(&mut flex, SizedBox::empty().with_auto_id());
    });
    harness.flush_records_of(target_tag);

    // We've added another child, now nobody gets text events when no widget is focused.
    harness.keyboard_type_chars("A");
    assert_matches!(harness.get_records_of(target_tag)[..], []);
}

#[test]
fn tab_focus() {
    let child_1 = WidgetTag::new("child_1");
    let child_2 = WidgetTag::new("child_2");
    let child_3 = WidgetTag::new("child_3");
    let child_4 = WidgetTag::new("child_4");
    let child_5 = WidgetTag::new("child_5");

    let parent = Flex::column()
        .with_child(NewWidget::new_with_tag(Button::with_text(""), child_1))
        .with_child(NewWidget::new_with_tag(Button::with_text(""), child_2))
        .with_child(NewWidget::new_with_tag(Button::with_text(""), child_3))
        .with_child(NewWidget::new_with_tag(Button::with_text(""), child_4))
        .with_child(NewWidget::new_with_tag(Button::with_text(""), child_5))
        .with_auto_id();

    let mut harness = TestHarness::create(default_property_set(), parent);

    let child_1_id = harness.get_widget_with_tag(child_1).id();
    let child_2_id = harness.get_widget_with_tag(child_2).id();
    let child_3_id = harness.get_widget_with_tag(child_3).id();
    let child_4_id = harness.get_widget_with_tag(child_4).id();
    let child_5_id = harness.get_widget_with_tag(child_5).id();

    assert_eq!(harness.focused_widget_id(), None);

    // Tab moves focus to the next focusable widget in the tree.
    harness.focus_on(Some(child_2_id));
    harness.press_tab_key(false);
    assert_eq!(harness.focused_widget_id(), Some(child_3_id));

    // Shift+Tab moves focus to the previous focusable widget in the tree.
    harness.focus_on(Some(child_4_id));
    harness.press_tab_key(true);
    assert_eq!(harness.focused_widget_id(), Some(child_3_id));

    // When nothing is focused, Tab focuses the first focusable widget in the tree.
    harness.focus_on(None);
    harness.press_tab_key(false);
    assert_eq!(harness.focused_widget_id(), Some(child_1_id));

    // When nothing is focused, Shift+Tab focuses the last focusable widget in the tree.
    harness.focus_on(None);
    harness.press_tab_key(true);
    assert_eq!(harness.focused_widget_id(), Some(child_5_id));
}

// ACCESS EVENTS

#[test]
fn access_event_bubbling() {
    let target_tag = WidgetTag::new("target");
    let parent_tag = WidgetTag::new("parent");
    let grandparent_tag = WidgetTag::new("grandparent");

    let target = NewWidget::new_with_tag(ModularWidget::new(()).record(), target_tag);
    let parent = NewWidget::new_with_tag(ModularWidget::new_parent(target).record(), parent_tag);
    let grandparent =
        NewWidget::new_with_tag(ModularWidget::new_parent(parent).record(), grandparent_tag);

    let mut harness = TestHarness::create(default_property_set(), grandparent);
    let target_id = harness.get_widget_with_tag(target_tag).id();

    // Send random event
    harness.process_access_event(ActionRequest {
        action: accesskit::Action::Click,
        target: target_id.into(),
        data: None,
    });

    let has_access_event = |records: Vec<_>| {
        records.iter().any(|r| {
            matches!(
                r,
                Record::AccessEvent(AccessEvent {
                    action: accesskit::Action::Click,
                    data: None
                })
            )
        })
    };

    assert!(has_access_event(harness.get_records_of(target_tag)));
    assert!(has_access_event(harness.get_records_of(parent_tag)));
    assert!(has_access_event(harness.get_records_of(grandparent_tag)));
}

#[test]
fn accessibility_focus() {
    let child_2 = WidgetTag::new("child_2");
    let child_3 = WidgetTag::new("child_3");

    let parent = Flex::column()
        .with_child(NewWidget::new(Button::with_text("")))
        .with_child(NewWidget::new_with_tag(Button::with_text(""), child_2))
        .with_child(NewWidget::new_with_tag(Button::with_text(""), child_3))
        .with_child(NewWidget::new(Button::with_text("")))
        .with_auto_id();

    let mut harness = TestHarness::create(default_property_set(), parent);
    let child_2_id = harness.get_widget_with_tag(child_2).id();
    let child_3_id = harness.get_widget_with_tag(child_3).id();

    // Send focus event
    harness.process_access_event(ActionRequest {
        action: accesskit::Action::Focus,
        target: child_3_id.into(),
        data: None,
    });
    assert_eq!(harness.focused_widget_id(), Some(child_3_id));

    // Send blur event with incorrect id
    harness.process_access_event(ActionRequest {
        action: accesskit::Action::Blur,
        target: child_2_id.into(),
        data: None,
    });
    assert_eq!(harness.focused_widget_id(), Some(child_3_id));

    // Send blur event with correct id
    harness.process_access_event(ActionRequest {
        action: accesskit::Action::Blur,
        target: child_3_id.into(),
        data: None,
    });
    assert_eq!(harness.focused_widget_id(), None);
}
