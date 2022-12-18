// This software is licensed under Apache License 2.0 and distributed on an
// "as-is" basis without warranties of any kind. See the LICENSE file for
// details.

//! Tests related to propagation of invalid rects.

use crate::testing::{widget_ids, TestHarness};
use crate::widget::{Button, Flex};

#[test]
fn invalidate_union() {
    let [id_child_1, id_child_2] = widget_ids();

    let widget = Flex::column()
        .with_child_id(Button::new("hi"), id_child_1)
        .with_child_id(Button::new("there"), id_child_2);

    let mut harness = TestHarness::create(widget);

    // This resets the invalid region.
    let _ = harness.render();
    assert!(harness.window().invalid().is_empty());

    let child1_rect = harness.get_widget(id_child_1).state().layout_rect();
    let child2_rect = harness.get_widget(id_child_2).state().layout_rect();
    harness.mouse_move_to(id_child_1);
    assert_eq!(harness.window().invalid().rects(), &[child1_rect]);

    let _ = harness.render();
    assert!(harness.window().invalid().is_empty());

    harness.mouse_move_to(id_child_2);
    assert_eq!(
        harness.window().invalid().rects(),
        // TODO: this is probably too fragile, because is there any guarantee on the order?
        &[child1_rect, child2_rect]
    );
}

// TODO: Add a test with scrolling/viewport
