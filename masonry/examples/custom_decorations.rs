// Copyright 2024 the Xilem Authors and the Druid Authors
// SPDX-License-Identifier: Apache-2.0

// On Windows platform, don't show a console when opening the app.
#![windows_subsystem = "windows"]

use masonry::app_driver::{AppDriver, DriverCtx};
use masonry::dpi::LogicalSize;
use masonry::widget::{Button, CrossAxisAlignment, Flex, RootWidget, TitleBar, WindowDecorations};
use masonry::{Action, WidgetId};
use winit::window::Window;

struct Driver;

impl AppDriver for Driver {
    fn on_action(&mut self, _ctx: &mut DriverCtx<'_>, _widget_id: WidgetId, action: Action) {
        match action {
            Action::ButtonPressed(_) => {
                println!("Hello");
            }
            action => {
                eprintln!("Unexpected action {action:?}");
            }
        }
    }
}

pub fn main() {
    let title_bar = TitleBar::new();

    let button = Button::new("Say hello");

    let content = Flex::column()
        .cross_axis_alignment(CrossAxisAlignment::Fill)
        .with_child(title_bar)
        .with_flex_child(button, CrossAxisAlignment::Center);

    let main_widget = WindowDecorations::new(content);

    let window_size = LogicalSize::new(400.0, 400.0);
    let window_attributes = Window::default_attributes()
        .with_title("Hello World!")
        .with_resizable(true)
        .with_decorations(false)
        .with_min_inner_size(window_size);

    masonry::event_loop_runner::run(
        masonry::event_loop_runner::EventLoop::with_user_event(),
        window_attributes,
        RootWidget::new(main_widget),
        Driver,
    )
    .unwrap();
}
