// Copyright 2019 the Xilem Authors and the Druid Authors
// SPDX-License-Identifier: Apache-2.0

//! This is a very small example of how to setup a masonry application.
//! It does the almost bare minimum while still being useful.

// On Windows platform, don't show a console when opening the app.
#![windows_subsystem = "windows"]

use masonry::app_driver::{AppDriver, DriverCtx};
use masonry::widget::prelude::*;
use masonry::widget::{Button, Flex, Label};
use masonry::Action;
use winit::dpi::LogicalSize;
use winit::window::Window;

const VERTICAL_WIDGET_SPACING: f64 = 20.0;

struct Driver;

impl AppDriver for Driver {
    fn on_action(&mut self, _ctx: &mut DriverCtx<'_>, _widget_id: WidgetId, action: Action) {
        match action {
            Action::ButtonPressed => {
                println!("Hello");
            }
            action => {
                eprintln!("Unexpected action {action:?}");
            }
        }
    }
}

pub fn main() {
    let window_size = LogicalSize::new(400.0, 400.0);
    let window_attributes = Window::default_attributes()
        .with_title("Hello World!")
        .with_resizable(true)
        .with_min_inner_size(window_size);

    masonry::event_loop_runner::run(window_attributes, build_root_widget(), Driver).unwrap();
}

fn build_root_widget() -> impl Widget {
    let label = Label::new("Hello").with_text_size(32.0);

    // a button that says "hello"
    let button = Button::new("Say hello");

    // arrange the two widgets vertically, with some padding
    Flex::column()
        .with_child(label)
        .with_spacer(VERTICAL_WIDGET_SPACING)
        .with_child(button)
}
