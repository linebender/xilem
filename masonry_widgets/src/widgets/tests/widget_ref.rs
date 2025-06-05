// Copyright 2025 the Xilem Authors
// SPDX-License-Identifier: Apache-2.0

use assert_matches::assert_matches;

use crate::testing::{TestHarness, TestWidgetExt as _, widget_ids};
use crate::theme::default_property_set;
use crate::widgets::{Button, Label};

#[test]
fn downcast_ref_in_harness() {
    let [label_id] = widget_ids();
    let label = Label::new("Hello").with_id(label_id);

    let harness = TestHarness::create(default_property_set(), label);

    assert_matches!(harness.get_widget(label_id).downcast::<Label>(), Some(_));
    assert_matches!(harness.get_widget(label_id).downcast::<Button>(), None);
}
