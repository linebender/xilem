// Copyright 2019 the Xilem Authors and the Druid Authors
// SPDX-License-Identifier: Apache-2.0

//! This is a very small example of how to setup a masonry application.
//! It does the almost bare minimum while still being useful.

// On Windows platform, don't show a console when opening the app.
#![cfg_attr(not(test), windows_subsystem = "windows")]

use masonry::app::RenderRoot;
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
const ADD_BUTTON_TAG: WidgetTag<Button> = WidgetTag::named("add-button");
const LIST_TAG: WidgetTag<Flex> = WidgetTag::named("list");
const WIDGET_SPACING: Length = Length::const_px(5.0);

struct Driver {
    next_task: String,
    window_id: WindowId,
}

impl Driver {
    fn new() -> Self {
        Self {
            next_task: String::new(),
            window_id: WindowId::next(),
        }
    }

    fn ui_update(&mut self, render_root: &mut RenderRoot, action: ErasedAction) {
        if action.is::<ButtonPress>() {
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
                TextAction::Cancelled => {}
            }
        }
    }
}

impl AppDriver for Driver {
    fn on_action(
        &mut self,
        window_id: WindowId,
        ctx: &mut DriverCtx<'_>,
        _widget_id: WidgetId,
        action: ErasedAction,
    ) {
        debug_assert_eq!(window_id, self.window_id, "unknown window");

        self.ui_update(ctx.render_root(window_id), action);
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
                .with_props(PropertySet::new().with(Padding::all(WIDGET_SPACING))),
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
    let driver = Driver::new();

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
        let mut state = Driver::new();
        let mut harness = TestHarness::create(default_property_set(), make_widget_tree());
        let text_area_id = harness.get_widget(TEXT_INPUT_TAG).area_pod().id();
        let add_btn_id = harness.get_widget(ADD_BUTTON_TAG).id();

        assert_render_snapshot!(harness, "example_to_do_list_initial");

        let mut act_and_update =
            |harness: &mut TestHarness<_>, do_action: &dyn Fn(&mut TestHarness<_>)| {
                do_action(harness);
                // focus produces no actions while keyboard chars give multiple
                while let Some((erased_action, _)) = harness.pop_action_erased() {
                    state.ui_update(harness.render_root(), erased_action);
                }
            };

        act_and_update(&mut harness, &|harness| {
            harness.focus_on(Some(text_area_id));
        });
        act_and_update(&mut harness, &|harness| {
            harness.keyboard_type_chars("Buy milk");
        });
        assert_render_snapshot!(harness, "example_to_do_list_typed_task1");

        act_and_update(&mut harness, &|harness| {
            harness.mouse_click_on(add_btn_id, Some(PointerButton::Primary));
        });
        assert_render_snapshot!(harness, "example_to_do_list_task1_added");

        act_and_update(&mut harness, &|harness| {
            harness.focus_on(Some(text_area_id));
            harness.keyboard_type_chars("Do laundry");
            harness.mouse_click_on(add_btn_id, Some(PointerButton::Primary));
        });
        assert_render_snapshot!(harness, "example_to_do_list_task2_added");
    }
}
