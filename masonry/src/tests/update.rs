// Copyright 2025 the Xilem Authors
// SPDX-License-Identifier: Apache-2.0

use std::sync::mpsc;

use assert_matches::assert_matches;
use masonry_testing::{
    DebugName, ModularWidget, PRIMARY_MOUSE, Record, TestHarness, TestWidgetExt, assert_any,
    assert_debug_panics,
};

use crate::core::pointer::{PointerButton, PointerEvent};
use crate::core::{
    CursorIcon, Ime, NewWidget, PropertySet, TextEvent, Update, Widget, WidgetId, WidgetPod,
    WidgetTag,
};
use crate::layout::Length;
use crate::theme::test_property_set;
use crate::widgets::{Button, Flex, Label, SizedBox, TextArea};

// TREE

#[test]
fn app_creation() {
    let widget_tag = WidgetTag::named("widget");
    let widget = NewWidget::new_with_tag(SizedBox::empty().record(), widget_tag);

    let harness = TestHarness::create(test_property_set(), widget);

    assert_matches!(
        harness.take_records_of(widget_tag)[..],
        [
            Record::RegisterChildren,
            Record::Update(Update::WidgetAdded),
            Record::Layout(_),
            Record::Compose,
            Record::AnimFrame(0),
            Record::PrePaint,
            Record::Paint,
            Record::PostPaint,
            Record::Accessibility
        ]
    );
}

#[test]
fn new_widget() {
    let flex = NewWidget::new(Flex::column());

    let mut harness = TestHarness::create(test_property_set(), flex);

    let widget_tag = WidgetTag::named("widget");
    harness.edit_root_widget(|mut flex| {
        let widget = NewWidget::new_with_tag(SizedBox::empty().record(), widget_tag);

        Flex::add_fixed(&mut flex, widget);
    });

    assert_matches!(
        harness.take_records_of(widget_tag)[0..2],
        [
            Record::RegisterChildren,
            Record::Update(Update::WidgetAdded)
        ]
    );
}

#[test]
fn forget_register_child() {
    let widget = ModularWidget::new_parent(Flex::row().with_auto_id())
        .register_children_fn(|_child, _ctx| {
            // We forget to call ctx.register_child();
        })
        .with_auto_id();

    assert_debug_panics!(
        TestHarness::create(test_property_set(), widget),
        "did not call RegisterCtx::register_child()"
    );
}

#[test]
fn register_invalid_child() {
    let widget = ModularWidget::new_parent(Flex::row().with_auto_id())
        .register_children_fn(|child, ctx| {
            ctx.register_child(child);
            ctx.register_child(&mut WidgetPod::new(Flex::row()));
        })
        .with_auto_id();

    assert_debug_panics!(
        TestHarness::create(test_property_set(), widget),
        "in the list returned by children_ids"
    );
}

// DISABLED

#[test]
fn disabled_widget_gets_no_event() {
    let button_tag = WidgetTag::named("button");
    let parent_tag = WidgetTag::named("parent");
    let child = NewWidget::new_with_tag(Button::with_text("").record(), button_tag);
    let parent = NewWidget::new_with_tag(ModularWidget::new_parent(child), parent_tag);

    let mut harness = TestHarness::create(test_property_set(), parent);
    let button_id = harness.get_widget(button_tag).id();
    harness.focus_on(Some(button_id));
    harness.flush_records_of(button_tag);

    harness.set_disabled(button_tag, true);
    assert_matches!(
        harness.take_records_of(button_tag)[..],
        [
            Record::Update(Update::DisabledChanged(true)),
            Record::Update(Update::ChildFocusChanged(false)),
            Record::Update(Update::FocusChanged(false)),
        ]
    );

    harness.mouse_click_on(button_id);
    assert_matches!(harness.take_records_of(button_tag)[..], []);

    assert_matches!(harness.focused_widget_id(), None);

    // TODO - Try to request focus
}

#[test]
fn disable_parent() {
    let button_tag = WidgetTag::named("button");
    let parent_tag = WidgetTag::named("parent");
    let grandparent_tag = WidgetTag::named("grandparent_tag");
    let child = NewWidget::new_with_tag(Button::with_text("").record(), button_tag);
    let parent = NewWidget::new_with_tag(ModularWidget::new_parent(child), parent_tag);
    let grandparent = NewWidget::new_with_tag(ModularWidget::new_parent(parent), grandparent_tag);

    let mut harness = TestHarness::create(test_property_set(), grandparent);
    harness.flush_records_of(button_tag);

    // First we disable the parent: the button should get a "DisabledChanged" event.
    harness.set_disabled(parent_tag, true);
    assert_matches!(
        harness.take_records_of(button_tag)[..],
        [Record::Update(Update::DisabledChanged(true))]
    );

    assert!(harness.get_widget(button_tag).ctx().is_disabled());

    // Then we disable the grandparent: nothing should happen,
    // the parent is already disabled.
    harness.set_disabled(grandparent_tag, true);
    assert_matches!(harness.take_records_of(button_tag)[..], []);

    // Then we re-enable the parent: nothing should happen,
    // the parent is still disabled through the grandparent.
    harness.set_disabled(parent_tag, false);
    assert_matches!(harness.take_records_of(button_tag)[..], []);

    // Then we re-enable the grandparent: the button should get a "DisabledChanged" event.
    harness.set_disabled(grandparent_tag, false);
    assert_matches!(
        harness.take_records_of(button_tag)[..],
        [Record::Update(Update::DisabledChanged(false))]
    );

    // Finally we re-enable the button: no effect, it's already enabled.
    harness.set_disabled(button_tag, false);
    assert_matches!(harness.take_records_of(button_tag)[..], []);
}

// STASHED

#[test]
fn stashed_widget_loses_focus() {
    let button_tag = WidgetTag::named("button");
    let parent_tag = WidgetTag::named("parent");
    let child = NewWidget::new_with_tag(Button::with_text("").record(), button_tag);
    let parent = NewWidget::new_with_tag(ModularWidget::new_parent(child), parent_tag);

    let mut harness = TestHarness::create(test_property_set(), parent);
    let button_id = harness.get_widget(button_tag).id();
    harness.focus_on(Some(button_id));
    harness.flush_records_of(button_tag);

    harness.edit_widget(parent_tag, |mut widget| {
        widget.ctx.set_stashed(&mut widget.widget.state, true);
    });
    assert_matches!(
        harness.take_records_of(button_tag)[..],
        [
            Record::Update(Update::StashedChanged(true)),
            Record::Update(Update::ChildFocusChanged(false)),
            Record::Update(Update::FocusChanged(false)),
        ]
    );

    assert_matches!(harness.focused_widget_id(), None);

    // TODO - Try to request focus
}

#[test]
fn stash_parent() {
    let button_tag = WidgetTag::named("button");
    let parent_tag = WidgetTag::named("parent");
    let grandparent_tag = WidgetTag::named("grandparent_tag");
    let child = NewWidget::new_with_tag(Button::with_text("").record(), button_tag);
    let parent = NewWidget::new_with_tag(ModularWidget::new_parent(child), parent_tag);
    let grandparent = NewWidget::new_with_tag(ModularWidget::new_parent(parent), grandparent_tag);

    let mut harness = TestHarness::create(test_property_set(), grandparent);
    harness.flush_records_of(button_tag);

    // First we stash the button: the button should get a "StashedChanged" event.
    harness.edit_widget(parent_tag, |mut widget| {
        widget.ctx.set_stashed(&mut widget.widget.state, true);
    });
    assert_matches!(
        harness.take_records_of(button_tag)[..],
        [Record::Update(Update::StashedChanged(true))]
    );

    assert!(harness.get_widget(button_tag).ctx().is_stashed());

    // Then we stash the parent: nothing should happen,
    // the button is already stashed.
    harness.edit_widget(grandparent_tag, |mut widget| {
        widget.ctx.set_stashed(&mut widget.widget.state, true);
    });
    assert_matches!(harness.take_records_of(button_tag)[..], []);

    // Then we un-stash the button: nothing should happen,
    // the button is still stashed through the parent.
    harness.edit_widget(parent_tag, |mut widget| {
        widget.ctx.set_stashed(&mut widget.widget.state, false);
    });
    assert_matches!(harness.take_records_of(button_tag)[..], []);

    // Then we un-stash the parent: the button should get a "StashedChanged" event.
    harness.edit_widget(grandparent_tag, |mut widget| {
        widget.ctx.set_stashed(&mut widget.widget.state, false);
    });
    assert_matches!(
        harness.take_records_of(button_tag)[..],
        [
            Record::Update(Update::StashedChanged(false)),
            // Un-stashing also requests a layout pass.
            Record::Measure(_),
            Record::Measure(_),
            Record::Measure(_),
            Record::Measure(_),
            Record::Layout(_),
            Record::Compose
        ]
    );
}

// FOCUSABLE

fn focusable_child(name: &'static str) -> NewWidget<impl Widget> {
    NewWidget::new_with_props(
        ModularWidget::new(()).accepts_focus(true),
        PropertySet::one(DebugName(name.to_string())),
    )
}

fn focusable_parent(
    name: &'static str,
    children: Vec<NewWidget<impl Widget + ?Sized>>,
) -> NewWidget<impl Widget> {
    NewWidget::new_with_props(
        ModularWidget::new_multi_parent(children).accepts_focus(true),
        PropertySet::one(DebugName(name.to_string())),
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

    let mut harness = TestHarness::create(test_property_set(), root);

    fn get_name(harness: &TestHarness<impl Widget>, id: Option<WidgetId>) -> Option<String> {
        Some(
            harness
                .get_widget_with_id(id?)
                .get_prop::<DebugName>()
                .0
                .clone(),
        )
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
    let button1_tag = WidgetTag::named("button1");
    let button2_tag = WidgetTag::named("button2");
    let button3_tag = WidgetTag::named("button3");

    let button1 = NewWidget::new_with_tag(Button::with_text(""), button1_tag);
    let button2 = NewWidget::new_with_tag(Button::with_text(""), button2_tag);
    let button3 = NewWidget::new_with_tag(Button::with_text(""), button3_tag);

    let parent = NewWidget::new(ModularWidget::new_multi_parent(vec![
        button1, button2, button3,
    ]));

    let mut harness = TestHarness::create(test_property_set(), parent);

    let button1_id = harness.get_widget(button1_tag).id();
    let button2_id = harness.get_widget(button2_tag).id();
    let button3_id = harness.get_widget(button3_tag).id();

    harness.focus_on(Some(button2_id));
    harness.set_disabled(button2_tag, true);

    // We skip button2 and jump from button1 to button3.
    harness.focus_on(Some(button1_id));
    harness.press_tab_key(false);
    assert_eq!(harness.focused_widget_id(), Some(button3_id));

    // Same thing the other way.
    harness.press_tab_key(true);
    assert_eq!(harness.focused_widget_id(), Some(button1_id));
}

#[test]
fn stash_focusable() {
    let button1_tag = WidgetTag::named("button1");
    let button2_tag = WidgetTag::named("button2");
    let button3_tag = WidgetTag::named("button3");

    let button1 = NewWidget::new_with_tag(Button::with_text(""), button1_tag);
    let button2 = NewWidget::new_with_tag(Button::with_text(""), button2_tag);
    let button3 = NewWidget::new_with_tag(Button::with_text(""), button3_tag);

    let parent = NewWidget::new(ModularWidget::new_multi_parent(vec![
        button1, button2, button3,
    ]));

    let mut harness = TestHarness::create(test_property_set(), parent);

    let button1_id = harness.get_widget(button1_tag).id();
    let button2_id = harness.get_widget(button2_tag).id();
    let button3_id = harness.get_widget(button3_tag).id();

    harness.focus_on(Some(button2_id));

    harness.edit_root_widget(|mut parent| {
        parent.ctx.set_stashed(&mut parent.widget.state[1], true);
    });

    // We skip button2 and jump from button1 to button3.
    harness.focus_on(Some(button1_id));
    harness.press_tab_key(false);
    assert_eq!(harness.focused_widget_id(), Some(button3_id));

    // Same thing the other way.
    harness.press_tab_key(true);
    assert_eq!(harness.focused_widget_id(), Some(button1_id));
}

#[test]
fn remove_focusable() {
    let button1_tag = WidgetTag::named("button1");
    let button2_tag = WidgetTag::named("button2");
    let button3_tag = WidgetTag::named("button3");

    let button1 = NewWidget::new_with_tag(Button::with_text(""), button1_tag);
    let button2 = NewWidget::new_with_tag(Button::with_text(""), button2_tag);
    let button3 = NewWidget::new_with_tag(Button::with_text(""), button3_tag);

    let parent = NewWidget::new(ModularWidget::new_multi_parent(vec![
        button1, button2, button3,
    ]));

    let mut harness = TestHarness::create(test_property_set(), parent);

    let button1_id = harness.get_widget(button1_tag).id();
    let button2_id = harness.get_widget(button2_tag).id();
    let button3_id = harness.get_widget(button3_tag).id();

    harness.focus_on(Some(button2_id));
    harness.edit_root_widget(|mut parent| {
        let child = parent.widget.state.remove(1);
        parent.ctx.remove_child(child);
    });

    // We go from button1 to button3.
    harness.focus_on(Some(button1_id));
    harness.press_tab_key(false);
    assert_eq!(harness.focused_widget_id(), Some(button3_id));

    // Same thing the other way.
    harness.press_tab_key(true);
    assert_eq!(harness.focused_widget_id(), Some(button1_id));
}

// FOCUS

#[test]
fn ime_commit() {
    let textbox_tag = WidgetTag::named("textbox");
    let textbox = NewWidget::new_with_tag(TextArea::new_editable(""), textbox_tag);

    let mut harness = TestHarness::create(test_property_set(), textbox);
    let textbox_id = harness.get_widget(textbox_tag).id();

    harness.focus_on(Some(textbox_id));

    harness.process_text_event(TextEvent::Ime(Ime::Commit("New Text".to_string())));
    assert_eq!(harness.get_widget(textbox_tag).text(), "New Text");

    harness.process_text_event(TextEvent::Ime(Ime::Commit(" and more".to_string())));
    assert_eq!(harness.get_widget(textbox_tag).text(), "New Text and more");

    let ime_area_size = harness.ime_rect().1;
    assert!(ime_area_size.width > 0. && ime_area_size.height > 0.);
}

#[test]
fn ime_removed() {
    let textbox_tag = WidgetTag::named("textbox");
    let textbox = NewWidget::new_with_tag(TextArea::new_editable(""), textbox_tag);
    let parent = NewWidget::new(SizedBox::new(textbox));

    let mut harness = TestHarness::create(test_property_set(), parent);
    let textbox_id = harness.get_widget(textbox_tag).id();

    harness.focus_on(Some(textbox_id));

    harness.edit_root_widget(|mut sized_box| {
        SizedBox::remove_child(&mut sized_box);
    });

    assert!(!harness.has_ime_session());
    assert_matches!(harness.focused_widget_id(), None);
}

#[test]
fn ime_start_stop() {
    let textbox_tag = WidgetTag::named("textbox");
    let textbox = NewWidget::new_with_tag(TextArea::new_editable("").record(), textbox_tag);
    let parent = NewWidget::new(ModularWidget::new_parent(textbox));

    let mut harness = TestHarness::create(test_property_set(), parent);
    let textbox_id = harness.get_widget(textbox_tag).id();

    harness.focus_on(Some(textbox_id));

    assert!(harness.has_ime_session());

    harness.flush_records_of(textbox_tag);
    harness.set_disabled(textbox_tag, true);

    let records = harness.take_records_of(textbox_tag);
    assert_any(records, |r| {
        matches!(r, Record::TextEvent(TextEvent::Ime(Ime::Disabled)))
    });

    assert!(!harness.has_ime_session());
}

// SCROLL

// POINTER

fn create_icon_widget() -> ModularWidget<()> {
    ModularWidget::new(())
        .pointer_event_fn(|_, ctx, _, event| {
            if matches!(event, PointerEvent::Down { .. }) {
                ctx.capture_pointer();
            }
        })
        .cursor_icon(CursorIcon::Crosshair)
        .measure_fn(|_, _, _, _, _, _| 10.)
}

#[test]
fn cursor_icon() {
    let icon_tag = WidgetTag::named("icon");
    let label = NewWidget::new(Button::with_text("hello"));
    let icon_widget = NewWidget::new_with_tag(create_icon_widget(), icon_tag);
    let parent = NewWidget::new(Flex::row().with_fixed(label).with_fixed(icon_widget));

    let mut harness = TestHarness::create(test_property_set(), parent);
    let icon_id = harness.get_widget(icon_tag).id();

    assert_eq!(harness.cursor_icon(), CursorIcon::Default);

    harness.mouse_move_to(icon_id);
    assert_eq!(harness.cursor_icon(), CursorIcon::Crosshair);
}

#[test]
fn pointer_capture_affects_pointer_icon() {
    let label_tag = WidgetTag::named("label");
    let icon_tag = WidgetTag::named("icon");
    let label = NewWidget::new_with_tag(Button::with_text("hello"), label_tag);
    let icon_widget = NewWidget::new_with_tag(create_icon_widget(), icon_tag);
    let parent = NewWidget::new(Flex::row().with_fixed(label).with_fixed(icon_widget));

    let mut harness = TestHarness::create(test_property_set(), parent);
    let icon_id = harness.get_widget(icon_tag).id();
    let label_id = harness.get_widget(label_tag).id();

    harness.mouse_move_to(icon_id);
    harness.mouse_button_press(PointerButton::Primary);
    assert_eq!(harness.cursor_icon(), CursorIcon::Crosshair);

    // We keep the Crosshair icon as long as the pointer stays captured.
    harness.mouse_move_to(label_id);
    assert_eq!(harness.cursor_icon(), CursorIcon::Crosshair);

    harness.mouse_button_release(PointerButton::Primary);
    assert_eq!(harness.cursor_icon(), CursorIcon::Default);
}

#[test]
fn lose_hovered_on_pointer_leave_or_cancel() {
    let button_tag = WidgetTag::named("button");

    let button = NewWidget::new_with_tag(Button::with_text("button").record(), button_tag);

    let mut harness = TestHarness::create(test_property_set(), button);
    let button_id = harness.get_widget(button_tag).id();

    // Hover button
    harness.mouse_move_to(button_id);
    assert!(harness.get_widget(button_tag).ctx().is_hovered());

    // POINTER LEAVE
    harness.flush_records_of(button_tag);
    harness.process_pointer_event(PointerEvent::Leave(PRIMARY_MOUSE));

    assert!(!harness.get_widget(button_tag).ctx().is_hovered());

    let records = harness.take_records_of(button_tag);
    assert_any(records, |r| {
        matches!(r, Record::Update(Update::HoveredChanged(false)))
    });

    // Hover button again
    harness.mouse_move_to(button_id);
    assert!(harness.get_widget(button_tag).ctx().is_hovered());

    // POINTER CANCEL
    harness.flush_records_of(button_tag);
    harness.process_pointer_event(PointerEvent::Cancel(PRIMARY_MOUSE));

    assert!(!harness.get_widget(button_tag).ctx().is_hovered());

    let records = harness.take_records_of(button_tag);
    assert_any(records, |r| {
        matches!(r, Record::Update(Update::HoveredChanged(false)))
    });
}

#[test]
fn change_hovered_when_widget_changes() {
    const BOX_SIZE: Length = Length::const_px(50.);

    let child_tag = WidgetTag::named("child");
    let parent_tag = WidgetTag::named("parent");

    let child = NewWidget::new_with_tag(
        ModularWidget::new(BOX_SIZE).measure_fn(|size, _, _, _, _, _| size.get()),
        child_tag,
    );
    let parent = NewWidget::new_with_tag(
        ModularWidget::new_parent(child).measure_fn(|_, _, _, _, _, _| BOX_SIZE.get()),
        parent_tag,
    );

    let mut harness = TestHarness::create(test_property_set(), parent);
    let child_id = harness.get_widget(child_tag).id();

    harness.mouse_move_to(child_id);
    assert!(harness.get_widget(child_tag).ctx().is_hovered());
    assert!(!harness.get_widget(parent_tag).ctx().is_hovered());

    harness.edit_widget(child_tag, |mut child| {
        child.widget.state = Length::ZERO;
        child.ctx.request_layout();
    });

    // The pointer hasn't moved, but no longer covers the child.
    // The parent should now be the widget which is hovered.
    assert!(!harness.get_widget(child_tag).ctx().is_hovered());
    assert!(harness.get_widget(parent_tag).ctx().is_hovered());

    harness.edit_widget(child_tag, |mut child| {
        child.widget.state = BOX_SIZE;
        child.ctx.request_layout();
    });
    // We reverted the child to the old size. It should be hovered again.
    assert!(harness.get_widget(child_tag).ctx().is_hovered());
    assert!(!harness.get_widget(parent_tag).ctx().is_hovered());
}

// STATUS FLAGS

fn make_reporter_parent(
    child: NewWidget<impl Widget>,
    sender: mpsc::Sender<(String, u32)>,
    n: u32,
) -> impl Widget {
    ModularWidget::new_parent(child)
        .accepts_focus(true)
        .pointer_event_fn(|_, ctx, _, event| {
            if matches!(event, PointerEvent::Down { .. }) {
                // Makes widget active
                ctx.capture_pointer();
                ctx.set_handled();
            }
        })
        .measure_fn(|_, _, _, _, _, _| 100.)
        .update_fn(move |_, _, _, event| {
            sender.send((event.short_name().to_string(), n)).unwrap();
        })
}

#[test]
fn status_flag_update_order() {
    let (sender, receiver) = mpsc::channel::<(String, u32)>();
    let sender1 = sender.clone();
    let sender2 = sender.clone();
    let sender3 = sender;

    let parent1_tag = WidgetTag::named("parent1");

    let child = NewWidget::new(Label::new(""));
    let parent1 = NewWidget::new_with_tag(make_reporter_parent(child, sender1, 1), parent1_tag);
    let parent2 = NewWidget::new(make_reporter_parent(parent1, sender2, 2));
    let parent3 = NewWidget::new(make_reporter_parent(parent2, sender3, 3));

    let mut harness = TestHarness::create(test_property_set(), parent3);
    let parent1_id = harness.get_widget(parent1_tag).id();
    // Flush initial events
    let _ = receiver.try_iter().count();

    harness.mouse_move_to(parent1_id);
    let events: Vec<_> = receiver.try_iter().collect();
    assert_eq!(
        events,
        [
            ("ChildHoveredChanged(true)".into(), 1),
            ("ChildHoveredChanged(true)".into(), 2),
            ("ChildHoveredChanged(true)".into(), 3),
            ("HoveredChanged(true)".into(), 1)
        ]
    );
    assert!(harness.get_widget(parent1_tag).ctx().is_hovered());
    assert!(harness.get_widget(parent1_tag).ctx().has_hovered());

    harness.mouse_button_press(PointerButton::Primary);
    let events: Vec<_> = receiver.try_iter().collect();
    assert_eq!(
        events,
        [
            ("ChildActiveChanged(true)".into(), 1),
            ("ChildActiveChanged(true)".into(), 2),
            ("ChildActiveChanged(true)".into(), 3),
            ("ActiveChanged(true)".into(), 1)
        ]
    );
    assert!(harness.get_widget(parent1_tag).ctx().is_active());
    assert!(harness.get_widget(parent1_tag).ctx().has_active());

    harness.mouse_button_release(PointerButton::Primary);
    let events: Vec<_> = receiver.try_iter().collect();
    assert_eq!(
        events,
        [
            ("ChildActiveChanged(false)".into(), 1),
            ("ChildActiveChanged(false)".into(), 2),
            ("ChildActiveChanged(false)".into(), 3),
            ("ActiveChanged(false)".into(), 1)
        ]
    );
    assert!(!harness.get_widget(parent1_tag).ctx().is_active());
    assert!(!harness.get_widget(parent1_tag).ctx().has_active());

    harness.mouse_move((-10., -10.));
    let events: Vec<_> = receiver.try_iter().collect();
    assert_eq!(
        events,
        [
            ("ChildHoveredChanged(false)".into(), 1),
            ("ChildHoveredChanged(false)".into(), 2),
            ("ChildHoveredChanged(false)".into(), 3),
            ("HoveredChanged(false)".into(), 1)
        ]
    );
    assert!(!harness.get_widget(parent1_tag).ctx().is_hovered());
    assert!(!harness.get_widget(parent1_tag).ctx().has_hovered());

    harness.focus_on(Some(parent1_id));
    let events: Vec<_> = receiver.try_iter().collect();
    assert_eq!(
        events,
        [
            ("ChildFocusChanged(true)".into(), 1),
            ("ChildFocusChanged(true)".into(), 2),
            ("ChildFocusChanged(true)".into(), 3),
            ("FocusChanged(true)".into(), 1)
        ]
    );
    assert!(harness.get_widget(parent1_tag).ctx().is_focus_target());
    assert!(harness.get_widget(parent1_tag).ctx().has_focus_target());

    harness.focus_on(None);
    let events: Vec<_> = receiver.try_iter().collect();
    assert_eq!(
        events,
        [
            ("ChildFocusChanged(false)".into(), 1),
            ("ChildFocusChanged(false)".into(), 2),
            ("ChildFocusChanged(false)".into(), 3),
            ("FocusChanged(false)".into(), 1)
        ]
    );
    assert!(!harness.get_widget(parent1_tag).ctx().is_focus_target());
    assert!(!harness.get_widget(parent1_tag).ctx().has_focus_target());
}
