// Copyright 2019 the Xilem Authors and the Druid Authors
// SPDX-License-Identifier: Apache-2.0

//! This is a very small example of how to setup a masonry application.
//! It does the almost bare minimum while still being useful.

// On Windows platform, don't show a console when opening the app.
#![cfg_attr(not(test), windows_subsystem = "windows")]

use masonry::core::{ErasedAction, NewWidget, PropertySet, Widget, WidgetId, WidgetTag};
use masonry::dpi::LogicalSize;
use masonry::layout::Length;
use masonry::peniko::color::AlphaColor;
use masonry::properties::Padding;
use masonry::properties::types::CrossAxisAlignment;
use masonry::theme::default_property_set;
use masonry::widgets::{Button, ButtonPress, Flex, Label, Portal, TextAction, TextArea, TextInput};
use masonry_winit::app::{AppDriver, DriverCtx, NewWindow, WindowId};
use masonry_winit::winit::window::Window;

const TEXT_INPUT_TAG: WidgetTag<TextInput> = WidgetTag::named("text-input");
const LIST_TAG: WidgetTag<Flex> = WidgetTag::named("list");
const ADD_BUTTON_TAG: WidgetTag<Button> = WidgetTag::named("add-button");
const WIDGET_SPACING: Length = Length::const_px(5.0);

struct Driver {
    next_task: String,
    window_id: WindowId,
}

impl AppDriver for Driver {
    fn on_action(
        &mut self,
        window_id: WindowId,
        ctx: &mut DriverCtx<'_, '_>,
        _widget_id: WidgetId,
        action: ErasedAction,
    ) {
        debug_assert_eq!(window_id, self.window_id, "unknown window");

        if action.is::<ButtonPress>() {
            let render_root = ctx.render_root(window_id);

            render_root.edit_widget_with_tag(TEXT_INPUT_TAG, |mut text_input| {
                let mut text_area = TextInput::text_mut(&mut text_input);
                TextArea::reset_text(&mut text_area, "");
            });
            render_root.edit_widget_with_tag(LIST_TAG, |mut list| {
                let child = Label::new(self.next_task.clone()).prepare();
                Flex::add_fixed(&mut list, child);
            });
        } else if action.is::<TextAction>() {
            let action = action.downcast::<TextAction>().unwrap();
            match *action {
                TextAction::Changed(new_text) => {
                    self.next_task = new_text.clone();
                }
                TextAction::Entered(_) => {}
            }
        }
    }
}

/// Return initial to-do-list without items.
pub fn make_widget_tree() -> NewWidget<impl Widget> {
    let text_input = NewWidget::new(
        TextInput::new("").with_placeholder("ex: 'Do the dishes', 'File my taxes', ..."),
    )
    .with_tag(TEXT_INPUT_TAG);
    let button = NewWidget::new(Button::with_text("Add task")).with_tag(ADD_BUTTON_TAG);

    let portal = Portal::new(
        NewWidget::new(Flex::column().cross_axis_alignment(CrossAxisAlignment::Start))
            .with_tag(LIST_TAG),
    )
    .prepare();

    let root = Flex::column()
        .with_fixed(
            NewWidget::new(Flex::row().with(text_input, 1.0).with_fixed(button))
                .with_props(PropertySet::new().with(Padding::all(WIDGET_SPACING.get()))),
        )
        .with_fixed_spacer(WIDGET_SPACING)
        .with(portal, 1.0);

    NewWidget::new(root)
}

fn main() {
    let window_size = LogicalSize::new(400.0, 400.0);
    let window_attributes = Window::default_attributes()
        .with_title("To-do list")
        .with_resizable(true)
        .with_min_inner_size(window_size);
    let driver = Driver {
        next_task: String::new(),
        window_id: WindowId::next(),
    };

    let event_loop = masonry_winit::app::EventLoop::with_user_event()
        .build()
        .unwrap();
    masonry_winit::app::run_with(
        event_loop,
        vec![
            NewWindow::new_with_id(
                driver.window_id,
                window_attributes,
                make_widget_tree().erased(),
            )
            .with_base_color(AlphaColor::from_rgb8(2, 6, 23)),
        ],
        driver,
        default_property_set(),
    )
    .unwrap();
}

// --- MARK: TESTS
#[cfg(test)]
mod tests {
    use masonry::core::PointerButton;
    use masonry_testing::{TestHarness, assert_render_snapshot};

    use super::*;

    #[test]
    fn screenshot_test() {
        let mut harness = TestHarness::create(default_property_set(), make_widget_tree());
        assert_render_snapshot!(harness, "example_to_do_list_initial");
    }

    #[test]
    fn add_tasks() {
        let mut harness = TestHarness::create(default_property_set(), make_widget_tree());
        let text_input_id = harness.get_widget(TEXT_INPUT_TAG).id();
        let add_btn_id = harness.get_widget(ADD_BUTTON_TAG).id();

        // Type a task name, click Add, then replicate Driver::on_action's
        // work (clear the input, append a label). Re-focus on each call:
        // clicking Add can shift focus, so the next typed text would be lost.
        let add_task = |harness: &mut TestHarness<_>, task: &str| {
            harness.focus_on(Some(text_input_id));
            harness.keyboard_type_chars(task);
            harness.mouse_click_on(add_btn_id, Some(PointerButton::Primary));
            assert_eq!(
                harness.pop_action::<ButtonPress>(),
                Some((
                    ButtonPress {
                        button: Some(PointerButton::Primary),
                    },
                    add_btn_id,
                ))
            );

            harness.edit_widget(TEXT_INPUT_TAG, |mut text_input| {
                let mut text_area = TextInput::text_mut(&mut text_input);
                TextArea::reset_text(&mut text_area, "");
            });
            harness.edit_widget(LIST_TAG, |mut list| {
                Flex::add_fixed(&mut list, Label::new(task.to_string()).prepare());
            });
        };

        add_task(&mut harness, "Buy milk");
        assert_render_snapshot!(harness, "example_to_do_list_after_add");

        add_task(&mut harness, "Do laundry");
        assert_render_snapshot!(harness, "example_to_do_list_two_tasks");
    }
}
