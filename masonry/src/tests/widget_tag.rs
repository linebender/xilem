// Copyright 2025 the Xilem Authors
// SPDX-License-Identifier: Apache-2.0

use masonry_testing::{TestHarness, assert_debug_panics};

use crate::core::{NewWidget, WidgetTag};
use crate::theme::test_property_set;
use crate::widgets::{Flex, SizedBox};

#[test]
fn duplicate_widget_tag() {
    let tag = WidgetTag::named("hello");

    let target = NewWidget::new_with_tag(SizedBox::empty(), tag);
    let parent = NewWidget::new(Flex::row().with_fixed(target));

    let mut harness = TestHarness::create(test_property_set(), parent);

    assert_debug_panics!(
        harness.edit_root_widget(|mut flex| {
            let new_child = NewWidget::new_with_tag(SizedBox::empty(), tag);
            Flex::add_fixed(&mut flex, new_child);
        }),
        "already exists in the widget tree"
    );
}
