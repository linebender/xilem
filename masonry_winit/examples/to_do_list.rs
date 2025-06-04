// Copyright 2019 the Xilem Authors and the Druid Authors
// SPDX-License-Identifier: Apache-2.0

//! This is a very small example of how to setup a masonry application.
//! It does the almost bare minimum while still being useful.

// On Windows platform, don't show a console when opening the app.
#![cfg_attr(not(test), windows_subsystem = "windows")]

use masonry::core::WidgetPod;
use masonry_winit::app::{AppDriver, DriverCtx};
use masonry_winit::core::{Action, Widget, WidgetId};
use masonry_winit::dpi::LogicalSize;
use masonry_winit::widgets::{Button, Flex, Label, Portal, RootWidget, TextArea, Textbox};
use winit::window::Window;

const VERTICAL_WIDGET_SPACING: f64 = 20.0;

struct Driver {
    next_task: String,
    ids: WidgetIds,
}

#[derive(Default)]
struct WidgetIds {
    text_area: WidgetId,
    task_list_flex: WidgetId,
}

impl AppDriver for Driver {
    fn on_action(&mut self, ctx: &mut DriverCtx<'_>, _widget_id: WidgetId, action: Action) {
        match action {
            Action::ButtonPressed(_) => {
                ctx.render_root().edit_root_widget(|mut root| {
                    {
                        let mut task_list_flex = root.ctx.find_mut(self.ids.task_list_flex);
                        let mut task_list_flex = task_list_flex.downcast::<Flex>();
                        Flex::add_child(&mut task_list_flex, Label::new(self.next_task.clone()));
                    }
                    {
                        let mut text_area = root.ctx.find_mut(self.ids.text_area);
                        let mut text_area = text_area.downcast::<TextArea<true>>();
                        TextArea::reset_text(&mut text_area, "");
                    }
                });
            }
            Action::TextChanged(new_text) => {
                self.next_task = new_text.clone();
            }
            _ => {}
        }
    }
}

fn make_widget_tree(widget_ids: &WidgetIds) -> impl Widget {
    Portal::new_pod(WidgetPod::new_with_id(
        Flex::column()
            .with_child(
                Flex::row()
                    .with_flex_child(
                        Textbox::from_text_area_pod(WidgetPod::new_with_id(
                            TextArea::new_editable(""),
                            widget_ids.text_area,
                        )),
                        1.0,
                    )
                    .with_child(Button::new("Add task")),
            )
            .with_spacer(VERTICAL_WIDGET_SPACING),
        widget_ids.task_list_flex,
    ))
}

fn main() {
    let window_size = LogicalSize::new(400.0, 400.0);
    let window_attributes = Window::default_attributes()
        .with_title("To-do list")
        .with_resizable(true)
        .with_min_inner_size(window_size);

    let widget_ids = WidgetIds::default();

    masonry_winit::app::run(
        masonry_winit::app::EventLoop::with_user_event(),
        window_attributes,
        RootWidget::new(make_widget_tree(&widget_ids)),
        Driver {
            next_task: String::new(),
            ids: widget_ids,
        },
    )
    .unwrap();
}

// --- MARK: TESTS
#[cfg(test)]
mod tests {
    use masonry_winit::assert_render_snapshot;
    use masonry_winit::testing::TestHarness;
    use masonry_winit::theme::default_property_set;

    use super::*;

    #[test]
    fn screenshot_test() {
        let mut harness = TestHarness::create(
            default_property_set(),
            make_widget_tree(&WidgetIds::default()),
        );
        assert_render_snapshot!(harness, "example_to_do_list_initial");

        // TODO - Test clicking buttons
        // TODO - Test typing text
    }
}
