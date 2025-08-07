// Copyright 2025 the Xilem Authors
// SPDX-License-Identifier: Apache-2.0

use masonry_core::core::{NewWidget, WidgetTag};
use masonry_testing::{TestHarness, assert_debug_panics};

use crate::theme::default_property_set;
use crate::widgets::{Flex, SizedBox};

#[test]
fn duplicate_widget_tag() {
    let tag = WidgetTag::new("hello");

    let target = NewWidget::new_with_tag(SizedBox::empty(), tag);
    let parent = NewWidget::new(Flex::row().with_child(target));

    let mut harness = TestHarness::create(default_property_set(), parent);

    assert_debug_panics!(
        harness.edit_root_widget(|mut flex| {
            let new_child = NewWidget::new_with_tag(SizedBox::empty(), tag);
            Flex::add_child(&mut flex, new_child);
        }),
        "already exists in the widget tree"
    );
}
