// Copyright 2025 the Xilem Authors
// SPDX-License-Identifier: Apache-2.0

use assert_matches::assert_matches;
use masonry_core::core::{NewWidget, Update, WidgetTag};
use masonry_testing::{ModularWidget, Record, TestHarness, TestWidgetExt};
use ui_events::pointer::PointerButton;

use crate::theme::default_property_set;
use crate::widgets::Button;

// TREE

// DISABLED

#[test]
fn disabled_widget_gets_no_event() {
    let button_tag = WidgetTag::new("button");
    let parent_tag = WidgetTag::new("parent");
    let child = NewWidget::new_with_tag(Button::with_text("").record(), button_tag);
    let parent = NewWidget::new_with_tag(ModularWidget::new_parent(child), parent_tag);

    let mut harness = TestHarness::create(default_property_set(), parent);
    let button_id = harness.get_widget_with_tag(button_tag).id();
    harness.focus_on(Some(button_id));
    harness.flush_records_of(button_tag);

    harness.edit_widget_with_tag(button_tag, |mut widget| {
        widget.ctx.set_disabled(true);
    });
    assert_matches!(
        harness.get_records_of(button_tag)[..],
        [
            Record::Update(Update::DisabledChanged(true)),
            Record::Update(Update::ChildFocusChanged(false)),
            Record::Update(Update::FocusChanged(false)),
        ]
    );

    // Button isn't focused.
    assert_matches!(harness.focused_widget_id(), None);

    // Button isn't considered hovered even when mouse is on it.
    harness.mouse_move_to(button_id);
    assert!(!harness.get_widget_with_tag(button_tag).ctx().is_hovered());

    // Button isn't considered active even during mouse press.
    harness.mouse_button_press(PointerButton::Primary);
    assert!(!harness.get_widget_with_tag(button_tag).ctx().is_active());

    // Clicking doesn't do anything.
    harness.mouse_click_on(button_id);
    assert_matches!(harness.get_records_of(button_tag)[..], []);

    // TODO - Try to request focus
}

// STASHED

// FOCUS CHAIN

// FOCUS

// SCROLL

// POINTER
