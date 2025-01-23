// Copyright 2019 the Xilem Authors and the Druid Authors
// SPDX-License-Identifier: Apache-2.0

//! This is a very small example of how to setup a masonry application.
//! It does the almost bare minimum while still being useful.

// On Windows platform, don't show a console when opening the app.
#![windows_subsystem = "windows"]

use masonry::app::{AppDriver, DriverCtx};
use masonry::core::{Action, Widget, WidgetId};
use masonry::dpi::LogicalSize;
use masonry::widgets::{Button, Flex, Label, Portal, RootWidget, TextArea, Textbox};
use winit::window::Window;

const VERTICAL_WIDGET_SPACING: f64 = 20.0;

struct Driver {
    next_task: String,
}

impl AppDriver for Driver {
    fn on_action(&mut self, ctx: &mut DriverCtx<'_>, _widget_id: WidgetId, action: Action) {
        match action {
            Action::ButtonPressed(_) => {
                ctx.render_root().edit_root_widget(|mut root| {
                    let mut root = root.downcast::<RootWidget<Portal<Flex>>>();

                    let mut portal = RootWidget::child_mut(&mut root);
                    let mut flex = Portal::child_mut(&mut portal);
                    Flex::add_child(&mut flex, Label::new(self.next_task.clone()));

                    let mut first_row = Flex::child_mut(&mut flex, 0).unwrap();
                    let mut first_row = first_row.downcast::<Flex>();
                    let mut textbox = Flex::child_mut(&mut first_row, 0).unwrap();
                    let mut textbox = textbox.downcast::<Textbox>();
                    let mut text_area = Textbox::text_mut(&mut textbox);
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
            .with_child(
                Flex::row()
                    .with_flex_child(Textbox::new(""), 1.0)
                    .with_child(Button::new("Add task")),
            )
            .with_spacer(VERTICAL_WIDGET_SPACING),
    )
}

fn main() {
    let window_size = LogicalSize::new(400.0, 400.0);
    let window_attributes = Window::default_attributes()
        .with_title("To-do list")
        .with_resizable(true)
        .with_min_inner_size(window_size);

    masonry::app::run(
        masonry::app::EventLoop::with_user_event(),
        window_attributes,
        RootWidget::new(make_widget_tree()),
        Driver {
            next_task: String::new(),
        },
    )
    .unwrap();
}

// --- MARK: TESTS ---
#[cfg(test)]
mod tests {
    use insta::assert_debug_snapshot;
    use masonry::assert_render_snapshot;
    use masonry::testing::TestHarness;

    use super::*;

    #[test]
    fn screenshot_test() {
        let mut harness = TestHarness::create(make_widget_tree());
        assert_debug_snapshot!(harness.root_widget());
        assert_render_snapshot!(harness, "initial_screenshot");

        // TODO - Test clicking buttons
        // TODO - Test typing text
    }
}
