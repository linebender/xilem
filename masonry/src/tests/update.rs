// Copyright 2025 the Xilem Authors
// SPDX-License-Identifier: Apache-2.0

use masonry_core::core::{NewWidget, Properties, Widget, WidgetId};
use masonry_testing::{DebugName, ModularWidget, TestHarness};

use crate::theme::default_property_set;
use crate::widgets::SizedBox;

// TREE

// DISABLED

// STASHED

// FOCUSABLE

fn focusable_child(name: &'static str) -> NewWidget<impl Widget> {
    NewWidget::new_with_props(
        ModularWidget::new(()).accepts_focus(true),
        Properties::one(DebugName(name.to_string())),
    )
}

fn focusable_parent(
    name: &'static str,
    children: Vec<NewWidget<impl Widget + ?Sized>>,
) -> NewWidget<impl Widget> {
    NewWidget::new_with_props(
        ModularWidget::new_multi_parent(children).accepts_focus(true),
        Properties::one(DebugName(name.to_string())),
    )
}

#[test]
fn focus_order() {
    let child_a1 = focusable_child("child_a1");
    let child_a2 = focusable_child("child_a2");
    let child_a3 = focusable_child("child_a3");
    let child_b1 = focusable_child("child_b1");
    let child_c1 = focusable_child("child_c1");
    let child_c2 = NewWidget::new(SizedBox::empty());
    let child_d1 = NewWidget::new(SizedBox::empty());
    let child_d2 = focusable_child("child_d2");
    let parent_a = focusable_parent("parent_a", vec![child_a1, child_a2, child_a3]);
    let parent_b = focusable_parent("parent_b", vec![child_b1]);
    let parent_c = focusable_parent("parent_c", vec![child_c1.erased(), child_c2.erased()]);
    let parent_d = focusable_parent("parent_d", vec![child_d1.erased(), child_d2.erased()]);
    let root = focusable_parent(
        "root",
        vec![
            parent_a.erased(),
            parent_b.erased(),
            parent_c.erased(),
            parent_d.erased(),
        ],
    );

    // ORDER IS:
    // - root
    //   - parent_a
    //     - child_a1
    //     - child_a2
    //     - child_a3
    //   - parent_b
    //     - child_b1
    //   - parent_c
    //     - child_c1
    //         (- child_c2)
    //   - parent_d
    //         (- child_d1)
    //     - child_d2

    let mut harness = TestHarness::create(default_property_set(), root);

    fn get_name(harness: &TestHarness<impl Widget>, id: Option<WidgetId>) -> Option<String> {
        Some(harness.get_widget(id?).get_prop::<DebugName>().0.clone())
    }

    let mut focusable_widgets = Vec::new();
    harness.inspect_widgets(|widget| {
        if widget.ctx().accepts_focus() {
            let id = widget.ctx().widget_id();
            focusable_widgets.push(id);
        }
    });
    let mut next_focusable_widgets = focusable_widgets.clone();
    next_focusable_widgets.remove(0);
    next_focusable_widgets.push(*focusable_widgets.first().unwrap());

    for (&id, &next_id) in std::iter::zip(&focusable_widgets, &next_focusable_widgets) {
        harness.focus_on(Some(id));
        harness.press_tab_key(false);
        let focused = harness.focused_widget_id();
        assert_eq!(
            get_name(&harness, focused),
            get_name(&harness, Some(next_id)),
            "failed to find the right successor for {}",
            get_name(&harness, Some(id)).unwrap(),
        );
        assert_eq!(focused, Some(next_id));

        harness.press_tab_key(true);
        let focused = harness.focused_widget_id();
        assert_eq!(
            get_name(&harness, focused),
            get_name(&harness, Some(id)),
            "failed to find the right predecessor for {}",
            get_name(&harness, Some(next_id)).unwrap(),
        );
        assert_eq!(focused, Some(id));
    }

    harness.focus_on(None);
    harness.press_tab_key(false);
    let focused = harness.focused_widget_id();
    assert_eq!(
        get_name(&harness, focused),
        get_name(&harness, focusable_widgets.first().copied())
    );
    assert_eq!(focused, focusable_widgets.first().copied());

    harness.focus_on(None);
    harness.press_tab_key(true);
    let focused = harness.focused_widget_id();
    assert_eq!(
        get_name(&harness, focused),
        get_name(&harness, focusable_widgets.last().copied())
    );
    assert_eq!(focused, focusable_widgets.last().copied());
}

// FOCUS

// SCROLL

// POINTER
