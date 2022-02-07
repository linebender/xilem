// Copyright 2020 The Druid Authors.
//
// Licensed under the Apache License, Version 2.0 (the "License");
// you may not use this file except in compliance with the License.
// You may obtain a copy of the License at
//
//     http://www.apache.org/licenses/LICENSE-2.0
//
// Unless required by applicable law or agreed to in writing, software
// distributed under the License is distributed on an "AS IS" BASIS,
// WITHOUT WARRANTIES OR CONDITIONS OF ANY KIND, either express or implied.
// See the License for the specific language governing permissions and
// limitations under the License.

//! Tests related to propagation of invalid rects.

use crate::kurbo::Rect;
use crate::testing::{move_mouse, widget_ids, Harness, ModularWidget};
use crate::widget::{Button, Flex};
use crate::*;
use test_env_log::test;

#[test]
fn invalidate_union() {
    let [id_child_1, id_child_2] = widget_ids();

    let widget = Flex::column()
        .with_child_id(Button::new("hi"), id_child_1)
        .with_child_id(Button::new("there"), id_child_2);

    let mut harness = Harness::create(widget);

    let child1_rect = harness.get_widget(id_child_1).state().layout_rect();
    let child2_rect = harness.get_widget(id_child_2).state().layout_rect();
    harness.mouse_move_to(id_child_1);
    assert_eq!(harness.window().invalid().rects(), &[child1_rect]);

    // This resets the invalid region.
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
