// Copyright 2022 the Xilem Authors
// SPDX-License-Identifier: Apache-2.0

use crate::core::{PointerButton, Widget, WidgetId};
use crate::testing::{ModularWidget, TestHarness};
use crate::theme::default_property_set;

#[should_panic(expected = "event does not allow pointer capture")]
#[test]
#[cfg_attr(
    not(debug_assertions),
    ignore = "This test doesn't work without debug assertions (i.e. in release mode). See https://github.com/linebender/xilem/issues/477"
)]
fn check_pointer_capture_outside_pointer_down() {
    let widget = ModularWidget::new(())
        .pointer_event_fn(|_, ctx, _, _event| {
            ctx.capture_pointer();
        })
        .with_auto_id();

    let mut harness = TestHarness::create(default_property_set(), widget);
    harness.mouse_move((10.0, 10.0));
    harness.mouse_button_release(PointerButton::Primary);
}

#[should_panic(expected = "event does not allow pointer capture")]
#[test]
#[cfg_attr(
    not(debug_assertions),
    ignore = "This test doesn't work without debug assertions (i.e. in release mode). See https://github.com/linebender/xilem/issues/477"
)]
fn check_pointer_capture_text_event() {
    let id = WidgetId::next();
    let widget = ModularWidget::new(())
        .accepts_focus(true)
        .text_event_fn(|_, ctx, _, _event| {
            ctx.capture_pointer();
        })
        .with_id(id);

    let mut harness = TestHarness::create(default_property_set(), widget);
    harness.focus_on(Some(id));
    harness.keyboard_type_chars("a");
}
