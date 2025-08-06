// Copyright 2025 the Xilem Authors
// SPDX-License-Identifier: Apache-2.0

use assert_matches::assert_matches;
use masonry_core::core::{
    CursorIcon, Ime, NewWidget, TextEvent, Update, Widget, WidgetPod, WidgetTag,
};
use masonry_testing::{ModularWidget, Record, TestHarness, TestWidgetExt, assert_debug_panics};
use ui_events::pointer::{PointerButton, PointerEvent};
use vello::kurbo::Size;

use crate::theme::default_property_set;
use crate::widgets::{Button, Flex, SizedBox, TextArea};

// TREE

#[test]
fn app_creation() {
    let widget_tag = WidgetTag::new("widget");
    let widget = NewWidget::new_with_tag(SizedBox::empty().record(), widget_tag);

    let harness = TestHarness::create(default_property_set(), widget);

    assert_matches!(
        harness.get_records_of(widget_tag)[..],
        [
            Record::RegisterChildren,
            Record::Update(Update::WidgetAdded),
            Record::Layout(_),
            Record::Compose,
            Record::Layout(_),
            Record::Compose,
            Record::AnimFrame(0),
            Record::Paint,
            Record::PostPaint,
            Record::Accessibility
        ]
    );
}

#[test]
fn new_widget() {
    let flex = NewWidget::new(Flex::column());

    let mut harness = TestHarness::create(default_property_set(), flex);

    let widget_tag = WidgetTag::new("widget");
    harness.edit_root_widget(|mut flex| {
        let widget = NewWidget::new_with_tag(SizedBox::empty().record(), widget_tag);

        Flex::add_child(&mut flex, widget);
    });

    assert_matches!(
        harness.get_records_of(widget_tag)[0..2],
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
        TestHarness::create(default_property_set(), widget),
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
        TestHarness::create(default_property_set(), widget),
        "in the list returned by children_ids"
    );
}

// DISABLED

#[test]
fn disabled_widget_gets_no_event() {
    let button_tag = WidgetTag::new("button");
    let parent_tag = WidgetTag::new("parent");
    let child = NewWidget::new_with_tag(Button::with_text("").record(), button_tag);
    let parent = NewWidget::new_with_tag(ModularWidget::new_parent(child), parent_tag);

    let mut harness = TestHarness::create(default_property_set(), parent);
    let button_id = harness.get_widget_with_tag(button_tag).id();
    harness.focus_on(Some(button_id));
    harness.flush_records_of(button_tag);

    harness.edit_widget_with_tag(button_tag, |mut widget| {
        widget.ctx.set_disabled(true);
    });
    assert_matches!(
        harness.get_records_of(button_tag)[..],
        [
            Record::Update(Update::DisabledChanged(true)),
            Record::Update(Update::ChildFocusChanged(false)),
            Record::Update(Update::FocusChanged(false)),
        ]
    );

    #[cfg(false)]
    {
        // TODO - Suppress events for disabled widgets.
        // See https://github.com/linebender/xilem/pull/1224
        harness.mouse_click_on(button_id);
        assert_matches!(harness.get_records_of(button_tag)[..], []);
    }

    assert_matches!(harness.focused_widget_id(), None);

    // TODO - Try to request focus
}

#[test]
fn disable_parent() {
    let button_tag = WidgetTag::new("button");
    let parent_tag = WidgetTag::new("parent");
    let grandparent_tag = WidgetTag::new("grandparent_tag");
    let child = NewWidget::new_with_tag(Button::with_text("").record(), button_tag);
    let parent = NewWidget::new_with_tag(ModularWidget::new_parent(child), parent_tag);
    let grandparent = NewWidget::new_with_tag(ModularWidget::new_parent(parent), grandparent_tag);

    let mut harness = TestHarness::create(default_property_set(), grandparent);
    harness.flush_records_of(button_tag);

    // First we disable the parent: the button should get a "DisabledChanged" event.
    harness.edit_widget_with_tag(parent_tag, |mut widget| {
        widget.ctx.set_disabled(true);
    });
    assert_matches!(
        harness.get_records_of(button_tag)[..],
        [Record::Update(Update::DisabledChanged(true))]
    );

    assert!(harness.get_widget_with_tag(button_tag).ctx().is_disabled());

    // Then we disable the grandparent: nothing should happen,
    // the parent is already disabled.
    harness.edit_widget_with_tag(grandparent_tag, |mut widget| {
        widget.ctx.set_disabled(true);
    });
    assert_matches!(harness.get_records_of(button_tag)[..], []);

    // Then we re-enable the parent: nothing should happen,
    // the parent is still disabled through the grandparent.
    harness.edit_widget_with_tag(parent_tag, |mut widget| {
        widget.ctx.set_disabled(false);
    });
    assert_matches!(harness.get_records_of(button_tag)[..], []);

    // Then we re-enable the grandparent: the button should get a "DisabledChanged" event.
    harness.edit_widget_with_tag(grandparent_tag, |mut widget| {
        widget.ctx.set_disabled(false);
    });
    assert_matches!(
        harness.get_records_of(button_tag)[..],
        [Record::Update(Update::DisabledChanged(false))]
    );

    // Finally we re-enable the button: no effect, it's already enabled.
    harness.edit_widget_with_tag(button_tag, |mut widget| {
        widget.ctx.set_disabled(false);
    });
    assert_matches!(harness.get_records_of(button_tag)[..], []);
}

// STASHED

#[test]
fn stashed_widget_loses_focus() {
    let button_tag = WidgetTag::new("button");
    let parent_tag = WidgetTag::new("parent");
    let child = NewWidget::new_with_tag(Button::with_text("").record(), button_tag);
    let parent = NewWidget::new_with_tag(ModularWidget::new_parent(child), parent_tag);

    let mut harness = TestHarness::create(default_property_set(), parent);
    let button_id = harness.get_widget_with_tag(button_tag).id();
    harness.focus_on(Some(button_id));
    harness.flush_records_of(button_tag);

    harness.edit_widget_with_tag(parent_tag, |mut widget| {
        widget.ctx.set_stashed(&mut widget.widget.state, true);
    });
    assert_matches!(
        harness.get_records_of(button_tag)[..],
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
    let button_tag = WidgetTag::new("button");
    let parent_tag = WidgetTag::new("parent");
    let grandparent_tag = WidgetTag::new("grandparent_tag");
    let child = NewWidget::new_with_tag(Button::with_text("").record(), button_tag);
    let parent = NewWidget::new_with_tag(ModularWidget::new_parent(child), parent_tag);
    let grandparent = NewWidget::new_with_tag(ModularWidget::new_parent(parent), grandparent_tag);

    let mut harness = TestHarness::create(default_property_set(), grandparent);
    harness.flush_records_of(button_tag);

    // First we stash the button: the button should get a "StashedChanged" event.
    harness.edit_widget_with_tag(parent_tag, |mut widget| {
        widget.ctx.set_stashed(&mut widget.widget.state, true);
    });
    assert_matches!(
        harness.get_records_of(button_tag)[..],
        [Record::Update(Update::StashedChanged(true))]
    );

    assert!(harness.get_widget_with_tag(button_tag).ctx().is_stashed());

    // Then we stash the parent: nothing should happen,
    // the button is already stashed.
    harness.edit_widget_with_tag(grandparent_tag, |mut widget| {
        widget.ctx.set_stashed(&mut widget.widget.state, true);
    });
    assert_matches!(harness.get_records_of(button_tag)[..], []);

    // Then we un-stash the button: nothing should happen,
    // the button is still stashed through the parent.
    harness.edit_widget_with_tag(parent_tag, |mut widget| {
        widget.ctx.set_stashed(&mut widget.widget.state, false);
    });
    assert_matches!(harness.get_records_of(button_tag)[..], []);

    // Then we un-stash the parent: the button should get a "StashedChanged" event.
    harness.edit_widget_with_tag(grandparent_tag, |mut widget| {
        widget.ctx.set_stashed(&mut widget.widget.state, false);
    });
    assert_matches!(
        harness.get_records_of(button_tag)[..],
        [
            Record::Update(Update::StashedChanged(false)),
            // Un-stashing also requests a layout pass.
            Record::Layout(_),
            Record::Compose
        ]
    );
}

// FOCUS CHAIN

// FOCUS

#[test]
fn ime_commit() {
    let textbox_tag = WidgetTag::new("textbox");
    let textbox = NewWidget::new_with_tag(TextArea::new_editable(""), textbox_tag);

    let mut harness = TestHarness::create(default_property_set(), textbox);
    let textbox_id = harness.get_widget_with_tag(textbox_tag).id();

    harness.focus_on(Some(textbox_id));

    harness.process_text_event(TextEvent::Ime(Ime::Commit("New Text".to_string())));
    assert_eq!(harness.get_widget_with_tag(textbox_tag).text(), "New Text");

    harness.process_text_event(TextEvent::Ime(Ime::Commit(" and more".to_string())));
    assert_eq!(
        harness.get_widget_with_tag(textbox_tag).text(),
        "New Text and more"
    );

    let ime_area_size = harness.ime_rect().1;
    assert!(ime_area_size.width > 0. && ime_area_size.height > 0.);
}

#[test]
fn ime_removed() {
    let textbox_tag = WidgetTag::new("textbox");
    let textbox = NewWidget::new_with_tag(TextArea::new_editable(""), textbox_tag);
    let parent = NewWidget::new(SizedBox::new(textbox));

    let mut harness = TestHarness::create(default_property_set(), parent);
    let textbox_id = harness.get_widget_with_tag(textbox_tag).id();

    harness.focus_on(Some(textbox_id));

    harness.edit_root_widget(|mut sized_box| {
        SizedBox::remove_child(&mut sized_box);
    });

    assert!(!harness.has_ime_session());
    assert_matches!(harness.focused_widget_id(), None);
}

#[test]
fn ime_start_stop() {
    let textbox_tag = WidgetTag::new("textbox");
    let textbox = NewWidget::new_with_tag(TextArea::new_editable("").record(), textbox_tag);
    let parent = NewWidget::new(ModularWidget::new_parent(textbox));

    let mut harness = TestHarness::create(default_property_set(), parent);
    let textbox_id = harness.get_widget_with_tag(textbox_tag).id();

    harness.focus_on(Some(textbox_id));

    assert!(harness.has_ime_session());

    harness.flush_records_of(textbox_tag);
    harness.edit_widget_with_tag(textbox_tag, |mut widget| {
        widget.ctx.set_disabled(true);
    });

    let records = harness.get_records_of(textbox_tag);
    assert!(
        records
            .iter()
            .any(|r| matches!(r, Record::TextEvent(TextEvent::Ime(Ime::Disabled))))
    );

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
        .layout_fn(|_, _, _, _| Size::new(10., 10.))
}

#[test]
fn cursor_icon() {
    let icon_tag = WidgetTag::new("icon");
    let label = NewWidget::new(Button::with_text("hello"));
    let icon_widget = NewWidget::new_with_tag(create_icon_widget(), icon_tag);
    let parent = NewWidget::new(Flex::row().with_child(label).with_child(icon_widget));

    let mut harness = TestHarness::create(default_property_set(), parent);
    let icon_id = harness.get_widget_with_tag(icon_tag).id();

    assert_eq!(harness.cursor_icon(), CursorIcon::Default);

    harness.mouse_move_to(icon_id);
    assert_eq!(harness.cursor_icon(), CursorIcon::Crosshair);
}

#[test]
fn pointer_capture_affects_pointer_icon() {
    let label_tag = WidgetTag::new("label");
    let icon_tag = WidgetTag::new("icon");
    let label = NewWidget::new_with_tag(Button::with_text("hello"), label_tag);
    let icon_widget = NewWidget::new_with_tag(create_icon_widget(), icon_tag);
    let parent = NewWidget::new(Flex::row().with_child(label).with_child(icon_widget));

    let mut harness = TestHarness::create(default_property_set(), parent);
    let icon_id = harness.get_widget_with_tag(icon_tag).id();
    let label_id = harness.get_widget_with_tag(label_tag).id();

    harness.mouse_move_to(icon_id);
    harness.mouse_button_press(PointerButton::Primary);
    assert_eq!(harness.cursor_icon(), CursorIcon::Crosshair);

    // We keep the Crosshair icon as long as the pointer stays captured.
    harness.mouse_move_to(label_id);
    assert_eq!(harness.cursor_icon(), CursorIcon::Crosshair);

    harness.mouse_button_release(PointerButton::Primary);
    assert_eq!(harness.cursor_icon(), CursorIcon::Default);
}
