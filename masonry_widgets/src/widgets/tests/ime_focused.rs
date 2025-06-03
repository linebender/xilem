// Copyright 2025 the Xilem Authors
// SPDX-License-Identifier: Apache-2.0

use crate::core::{Ime, TextEvent, WidgetPod};
use crate::testing::{TestHarness, widget_ids};
use crate::theme::default_property_set;
use crate::widgets::{Flex, TextArea, Textbox};

/// Tests that IME's interactions with focus are sensible.

#[test]
fn ime_on_remove() {
    let [text_area] = widget_ids();
    let widget = Flex::column().with_child(Textbox::from_text_area_pod(WidgetPod::new_with_id(
        TextArea::new_editable("Simple input test"),
        text_area,
    )));

    let mut harness = TestHarness::create(default_property_set(), widget);
    harness.focus_on(Some(text_area));
    harness.process_text_event(TextEvent::Ime(Ime::Commit("New Text".to_string())));
    let text_area = harness
        .get_widget(text_area)
        .downcast::<TextArea<true>>()
        .unwrap();
    // TODO: Ideally the cursor would start at the logical end of the text.
    assert_eq!(text_area.text(), "New TextSimple input test");
    let ime_area_size = harness.ime_rect().1;
    assert!(ime_area_size.width > 0. && ime_area_size.height > 0.);
    harness.edit_root_widget(|mut widget| {
        let mut widget = widget.downcast::<Flex>();
        Flex::remove_child(&mut widget, 0);
    });
}
