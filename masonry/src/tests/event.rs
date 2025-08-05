// Copyright 2025 the Xilem Authors
// SPDX-License-Identifier: Apache-2.0

// POINTER EVENTS

use assert_matches::assert_matches;
use masonry_core::core::{NewWidget, TextEvent, Widget, WidgetTag};
use masonry_testing::{ModularWidget, Record, TestHarness, TestWidgetExt};
use ui_events::keyboard::{Key, NamedKey};
use ui_events::pointer::{PointerButton, PointerEvent, PointerInfo, PointerType};
use vello::kurbo::Size;

use crate::theme::default_property_set;
use crate::widgets::{Button, Flex, SizedBox};

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

    // As long as 'target' is captured, 'other' doesn't get pointer events, event when the cursor is on it.
    harness.mouse_move_to(other_id);
    assert_matches!(harness.get_records_of(other_tag)[..], []);

    // 'other' is considered hovered either.
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

// ACCESS EVENTS
