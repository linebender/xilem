// Copyright 2025 the Xilem Authors
// SPDX-License-Identifier: Apache-2.0

// POINTER EVENTS

use assert_matches::assert_matches;
use masonry_core::core::{NewWidget, Widget, WidgetTag};
use masonry_testing::{ModularWidget, TestHarness, TestWidgetExt};
use ui_events::pointer::{PointerButton, PointerEvent};
use vello::kurbo::Size;

use crate::theme::default_property_set;
use crate::widgets::{Flex, SizedBox};

#[test]
fn pointer_capture_suppresses_neighbors() {
    let target_tag = WidgetTag::new("target");
    let other_tag = WidgetTag::new("other");

    let target = ModularWidget::new(())
        .pointer_event_fn(|_, ctx, _, event| {
            if matches!(event, PointerEvent::Down { .. }) {
                ctx.capture_pointer();
            }
        })
        .layout_fn(|_, _, _, _| Size::new(10., 10.));
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

// TEXT EVENTS

// ACCESS EVENTS
