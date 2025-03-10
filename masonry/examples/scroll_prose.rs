// Copyright 2019 the Xilem Authors and the Druid Authors
// SPDX-License-Identifier: Apache-2.0

//! This is a very small example of how to setup a masonry application.
//! It does the almost bare minimum while still being useful.

// On Windows platform, don't show a console when opening the app.
#![windows_subsystem = "windows"]

use masonry::app::{AppDriver, DriverCtx};
use masonry::core::{Action, StyleProperty, WidgetId};
use masonry::dpi::LogicalSize;
use masonry::parley::style::FontWeight;
use masonry::widgets::{Button, Flex, Label, Portal, Prose, RootWidget, TextArea};
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

const LOREM: &str = r"Lorem ipsum dolor sit amet, consectetur adipiscing elit. Morbi cursus mi sed euismod euismod. Orci varius natoque penatibus et magnis dis parturient montes, nascetur ridiculus mus. Nullam placerat efficitur tellus at semper. Morbi ac risus magna. Donec ut cursus ex. Etiam quis posuere tellus. Mauris posuere dui et turpis mollis, vitae luctus tellus consectetur. Lorem ipsum dolor sit amet, consectetur adipiscing elit. Curabitur eu facilisis nisl.

Phasellus in viverra dolor, vitae facilisis est. Maecenas malesuada massa vel ultricies feugiat. Vivamus venenatis et nibh nec pharetra. Phasellus vestibulum elit enim, nec scelerisque orci faucibus id. Vivamus consequat purus sit amet orci egestas, non iaculis massa porttitor. Vestibulum ut eros leo. In fermentum convallis magna in finibus. Donec justo leo, maximus ac laoreet id, volutpat ut elit. Mauris sed leo non neque laoreet faucibus. Aliquam orci arcu, faucibus in molestie eget, ornare non dui. Donec volutpat nulla in fringilla elementum. Aliquam vitae ante egestas ligula tempus vestibulum sit amet sed ante.
";

fn main() {
    let text = LOREM.repeat(10);
    let prose = Prose::from_text_area(
        TextArea::new(&text)
            .with_style(StyleProperty::FontSize(32.0))
            // Ideally there's be an Into in Parley for this
            .with_style(StyleProperty::FontWeight(FontWeight::BOLD)),
    );
    let portal = Portal::new(prose);

    let window_size = LogicalSize::new(400.0, 400.0);
    let window_attributes = Window::default_attributes()
        .with_title("Portal with Prose Test!")
        .with_resizable(true)
        .with_min_inner_size(window_size);

    masonry::app::run(
        masonry::app::EventLoop::with_user_event(),
        window_attributes,
        RootWidget::new(portal),
        Driver,
    )
    .unwrap();
}
