// Copyright 2021 the Xilem Authors
// SPDX-License-Identifier: Apache-2.0

#![expect(unused, reason = "Lots of code cfg-ed out")]

use std::cell::Cell;
use std::rc::Rc;

use crate::testing::{ModularWidget, TestHarness, TestWidgetExt as _, WrapperWidget, widget_ids};
use crate::widgets::Flex;
use crate::*;

#[cfg(false)]
/// Check that focus changes trigger on_status_change
#[test]
fn focus_status_change() {
    let [id_1, id_2] = widget_ids();

    // we use these so that we can check that on_status_check was called
    let left_focus: Rc<Cell<bool>> = Default::default();
    let right_focus: Rc<Cell<bool>> = Default::default();
    assert_eq!(left_focus.get(), false);
    assert_eq!(right_focus.get(), false);

    let widget = Flex::row()
        .with_child_id(FocusTaker::track(left_focus.clone()), id_1)
        .with_child_id(FocusTaker::track(right_focus.clone()), id_2);

    let mut harness = TestHarness::create(widgets);

    // nobody should have focus
    assert_eq!(left_focus.get(), false);
    assert_eq!(right_focus.get(), false);

    harness.submit_command(REQUEST_FOCUS.to(id_1));
    // check that left widget got "on_status_change" event.
    assert_eq!(left_focus.get(), true);
    assert_eq!(right_focus.get(), false);

    harness.submit_command(REQUEST_FOCUS.to(id_2));
    // check that left and right widget got "on_status_change" event.
    assert_eq!(left_focus.get(), false);
    assert_eq!(right_focus.get(), true);
}

#[cfg(false)]
/// test that the last widget to request focus during an event gets it.
#[test]
fn take_focus() {
    let [id_1, id_2, id_3, id_4] = widget_ids();

    let widget = Flex::row()
        .with_child_id(FocusTaker::new(), id_1)
        .with_child_id(FocusTaker::new(), id_2)
        .with_child_id(FocusTaker::new(), id_3)
        .with_child_id(FocusTaker::new(), id_4);

    let mut harness = TestHarness::create(widgets);

    // nobody should have focus
    assert_eq!(harness.window().focus, None);

    // this is sent to all widgets; the last widget to request focus should get it
    harness.submit_command(REQUEST_FOCUS);
    assert_eq!(harness.window().focus, Some(id_4));

    // this is sent to all widgets; the last widget to request focus should still get it
    harness.submit_command(REQUEST_FOCUS);
    assert_eq!(harness.window().focus, Some(id_4));
}

#[cfg(false)]
#[test]
fn focus_updated_by_children_change() {
    let [id_1, id_2, id_3, id_4, id_5, id_6] = widget_ids();

    // this widget starts with a single child, and will replace them with a split
    // when we send it a command.
    let replacer = WrapperWidget::new(FocusTaker::new().with_id(id_4), move || {
        Flex::row()
            .with_child_id(FocusTaker::new(), id_5)
            .with_child_id(FocusTaker::new(), id_6)
    });

    let widget = Flex::row()
        .with_child_id(FocusTaker::new(), id_1)
        .with_child_id(FocusTaker::new(), id_2)
        .with_child_id(FocusTaker::new(), id_3)
        .with_child(replacer);

    let mut harness = TestHarness::create(widgets);

    // verify that we start out with four widgets registered for focus
    assert_eq!(harness.window().focus_chain(), &[id_1, id_2, id_3, id_4]);

    // tell the replacer widget to swap its children
    harness.submit_command(REPLACE_CHILD);

    // verify that the two new children are registered for focus.
    assert_eq!(
        harness.window().focus_chain(),
        &[id_1, id_2, id_3, id_5, id_6]
    );
}
