// Copyright 2025 the Xilem Authors
// SPDX-License-Identifier: Apache-2.0

use masonry_core::core::{NewWidget, Properties, Widget, WidgetId, WidgetTag};
use masonry_testing::{DebugName, ModularWidget, TestHarness};

use crate::theme::default_property_set;
use crate::widgets::{Button, SizedBox};

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

#[test]
fn disable_focusable() {
    let button1_tag = WidgetTag::new("button1");
    let button2_tag = WidgetTag::new("button2");
    let button3_tag = WidgetTag::new("button3");

    let button1 = NewWidget::new_with_tag(Button::with_text(""), button1_tag);
    let button2 = NewWidget::new_with_tag(Button::with_text(""), button2_tag);
    let button3 = NewWidget::new_with_tag(Button::with_text(""), button3_tag);

    let parent = NewWidget::new(ModularWidget::new_multi_parent(vec![
        button1, button2, button3,
    ]));

    let mut harness = TestHarness::create(default_property_set(), parent);

    let button1_id = harness.get_widget_with_tag(button1_tag).id();
    dbg!(button1_id);
    let button2_id = harness.get_widget_with_tag(button2_tag).id();
    dbg!(button2_id);
    let button3_id = harness.get_widget_with_tag(button3_tag).id();
    dbg!(button3_id);

    harness.focus_on(Some(button2_id));
    harness.edit_widget_with_tag(button2_tag, |mut button| {
        button.ctx.set_disabled(true);
    });

    // The focus anchor should have reset to the parent flex, so we select button1
    harness.press_tab_key(false);
    assert_eq!(harness.focused_widget_id(), Some(button1_id));

    // We skip button2 and jump from button1 to button3.
    harness.press_tab_key(false);
    assert_eq!(harness.focused_widget_id(), Some(button3_id));

    // Same thing the other way.
    harness.press_tab_key(true);
    assert_eq!(harness.focused_widget_id(), Some(button1_id));
}

#[test]
fn stash_focusable() {
    let button1_tag = WidgetTag::new("button1");
    let button2_tag = WidgetTag::new("button2");
    let button3_tag = WidgetTag::new("button3");

    let button1 = NewWidget::new_with_tag(Button::with_text(""), button1_tag);
    let button2 = NewWidget::new_with_tag(Button::with_text(""), button2_tag);
    let button3 = NewWidget::new_with_tag(Button::with_text(""), button3_tag);

    let parent = NewWidget::new(ModularWidget::new_multi_parent(vec![
        button1, button2, button3,
    ]));

    let mut harness = TestHarness::create(default_property_set(), parent);

    let button1_id = harness.get_widget_with_tag(button1_tag).id();
    let button2_id = harness.get_widget_with_tag(button2_tag).id();
    let button3_id = harness.get_widget_with_tag(button3_tag).id();

    harness.focus_on(Some(button2_id));

    harness.edit_root_widget(|mut parent| {
        parent.ctx.set_stashed(&mut parent.widget.state[1], true);
    });

    // The focus anchor should have reset to the parent flex, so we select button1
    harness.press_tab_key(false);
    assert_eq!(harness.focused_widget_id(), Some(button1_id));

    // We skip button2 and jump from button1 to button3.
    harness.press_tab_key(false);
    assert_eq!(harness.focused_widget_id(), Some(button3_id));

    // Same thing the other way.
    harness.press_tab_key(true);
    assert_eq!(harness.focused_widget_id(), Some(button1_id));
}

#[test]
fn remove_focusable() {
    let button1_tag = WidgetTag::new("button1");
    let button2_tag = WidgetTag::new("button2");
    let button3_tag = WidgetTag::new("button3");

    let button1 = NewWidget::new_with_tag(Button::with_text(""), button1_tag);
    let button2 = NewWidget::new_with_tag(Button::with_text(""), button2_tag);
    let button3 = NewWidget::new_with_tag(Button::with_text(""), button3_tag);

    let parent = NewWidget::new(ModularWidget::new_multi_parent(vec![
        button1, button2, button3,
    ]));

    let mut harness = TestHarness::create(default_property_set(), parent);

    let button1_id = harness.get_widget_with_tag(button1_tag).id();
    dbg!(button1_id);
    let button2_id = harness.get_widget_with_tag(button2_tag).id();
    dbg!(button2_id);
    let button3_id = harness.get_widget_with_tag(button3_tag).id();
    dbg!(button3_id);

    harness.focus_on(Some(button2_id));
    harness.edit_root_widget(|mut parent| {
        let child = parent.widget.state.remove(1);
        parent.ctx.remove_child(child);
    });

    // The focus anchor should have reset to the parent flex, so we select button1
    harness.press_tab_key(false);
    assert_eq!(harness.focused_widget_id(), Some(button1_id));

    // We go from button1 to button3.
    harness.press_tab_key(false);
    assert_eq!(harness.focused_widget_id(), Some(button3_id));

    // Same thing the other way.
    harness.press_tab_key(true);
    assert_eq!(harness.focused_widget_id(), Some(button1_id));
}

// FOCUS

// SCROLL

// POINTER
