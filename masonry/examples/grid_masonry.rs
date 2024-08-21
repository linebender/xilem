// Copyright 2019 the Xilem Authors and the Druid Authors
// SPDX-License-Identifier: Apache-2.0

//! This is a very small example of how to setup a masonry application.
//! It does the almost bare minimum while still being useful.

// On Windows platform, don't show a console when opening the app.
#![windows_subsystem = "windows"]

use masonry::app_driver::{AppDriver, DriverCtx};
use masonry::dpi::LogicalSize;
use masonry::widget::{Button, Grid, Label, RootWidget};
use masonry::{Action, PointerButton, WidgetId};
use winit::window::Window;

struct Driver {
    grid_spacing: f64,
}

impl AppDriver for Driver {
    fn on_action(&mut self, ctx: &mut DriverCtx<'_>, _widget_id: WidgetId, action: Action) {
        match action {
            Action::ButtonPressed(button) => {
                if button == PointerButton::Primary {
                    self.grid_spacing += 1.0;
                } else if button == PointerButton::Secondary {
                    self.grid_spacing -= 1.0;
                } else {
                    self.grid_spacing += 0.5;
                }

                ctx.get_root::<RootWidget<Grid>>()
                    .get_element().set_spacing(self.grid_spacing)
            }
            _ => ()
        }
    }
}

struct DrawnButton {
    x: i32,
    y: i32,
    width: i32,
    height: i32,
}

impl DrawnButton {
    fn get_label(&self) -> String {
        format!("X: {}, Y: {}, W: {}, H: {}", self.x, self.y, self.width, self.height)
    }
}

pub fn main() {
    let label = Label::new("Change spacing by right and\n left clicking on the buttons")
            .with_text_size(14.0);
    let button_inputs = vec![
        DrawnButton{ x: 0, y: 0, width: 1, height: 1 },
        DrawnButton{ x: 2, y: 0, width: 2, height: 1 },
        DrawnButton{ x: 0, y: 1, width: 1, height: 2 },
        DrawnButton{ x: 1, y: 1, width: 2, height: 2 },
        DrawnButton{ x: 3, y: 1, width: 1, height: 1 },
        DrawnButton{ x: 3, y: 2, width: 1, height: 1 },
        DrawnButton{ x: 0, y: 3, width: 4, height: 1 },
    ];

    let driver = Driver {
        grid_spacing: 1.0,
    };

    // Arrange the two widgets vertically, with some padding
    let mut main_widget = Grid::with_dimensions(4, 4)
        .with_spacing(driver.grid_spacing)
        .with_child(label, 1, 0, 1, 1);
    for button_input in button_inputs {
        let button = Button::new(button_input.get_label());
        main_widget = main_widget.with_child(
            button,
            button_input.x,
            button_input.y,
            button_input.width,
            button_input.height,
        )
    }

    let window_size = LogicalSize::new(800.0, 500.0);
    let window_attributes = Window::default_attributes()
        .with_title("Grid Layout")
        .with_resizable(true)
        .with_min_inner_size(window_size);

    masonry::event_loop_runner::run(
        masonry::event_loop_runner::EventLoop::with_user_event(),
        window_attributes,
        RootWidget::new(main_widget),
        driver,
    )
        .unwrap();
}
