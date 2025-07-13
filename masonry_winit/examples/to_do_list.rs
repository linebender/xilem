// Copyright 2019 the Xilem Authors and the Druid Authors
// SPDX-License-Identifier: Apache-2.0

//! This is a very small example of how to setup a masonry application.
//! It does the almost bare minimum while still being useful.

// On Windows platform, don't show a console when opening the app.
#![cfg_attr(not(test), windows_subsystem = "windows")]

use masonry::core::{Action, Properties, Widget, WidgetId, WidgetPod};
use masonry::dpi::LogicalSize;
use masonry::properties::Padding;
use masonry::theme::default_property_set;
use masonry::widgets::{Button, Flex, Label, Portal, TextArea, TextInput};
use masonry_winit::app::{AppDriver, DriverCtx, WindowId};
use masonry_winit::winit::window::Window;

const WIDGET_SPACING: f64 = 5.0;

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
        action: Action,
    ) {
        debug_assert_eq!(window_id, self.window_id, "unknown window");

        match action {
            Action::ButtonPressed(_) => {
                ctx.render_root(window_id).edit_root_widget(|mut root| {
                    let mut portal = root.downcast::<Portal<Flex>>();
                    let mut flex = Portal::child_mut(&mut portal);
                    Flex::add_child(&mut flex, Label::new(self.next_task.clone()));

                    let mut first_row = Flex::child_mut(&mut flex, 0).unwrap();
                    let mut first_row = first_row.downcast::<Flex>();
                    let mut text_input = Flex::child_mut(&mut first_row, 0).unwrap();
                    let mut text_input = text_input.downcast::<TextInput>();
                    let mut text_area = TextInput::text_mut(&mut text_input);
                    TextArea::reset_text(&mut text_area, "");
                });
            }
            Action::TextChanged(new_text) => {
                self.next_task = new_text.clone();
            }
            _ => {}
        }
    }
}

fn make_widget_tree() -> impl Widget {
    Portal::new(
        Flex::column()
            .with_child_pod(
                WidgetPod::new_with_props(
                    Flex::row()
                        .with_flex_child(TextInput::new(""), 1.0)
                        .with_child(Button::new("Add task")),
                    Properties::new().with(Padding::all(WIDGET_SPACING)),
                )
                .erased(),
            )
            .with_spacer(WIDGET_SPACING),
    )
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
        vec![(
            driver.window_id,
            window_attributes,
            WidgetPod::new(make_widget_tree()).erased(),
        )],
        driver,
        default_property_set(),
    )
    .unwrap();
}

// --- MARK: TESTS
#[cfg(test)]
mod tests {
    use masonry_testing::{TestHarness, assert_render_snapshot};

    use super::*;

    #[test]
    fn screenshot_test() {
        let mut harness = TestHarness::create(default_property_set(), make_widget_tree());

        assert_render_snapshot!(harness, "example_to_do_list_initial");

        // TODO - Test clicking buttons
        // TODO - Test typing text
    }
}
