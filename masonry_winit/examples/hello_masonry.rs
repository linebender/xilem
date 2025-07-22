// Copyright 2019 the Xilem Authors and the Druid Authors
// SPDX-License-Identifier: Apache-2.0

//! This is a very small example of how to setup a masonry application.
//! It does the almost bare minimum while still being useful.

// On Windows platform, don't show a console when opening the app.
#![windows_subsystem = "windows"]

use masonry::core::{ErasedAction, NewWidget, StyleProperty, Widget as _, WidgetId};
use masonry::dpi::LogicalSize;
use masonry::parley::style::FontWeight;
use masonry::theme::default_property_set;
use masonry::widgets::{Button, ButtonPress, Flex, Label};
use masonry_winit::app::{AppDriver, DriverCtx, WindowId};
use masonry_winit::winit::window::Window;

const VERTICAL_WIDGET_SPACING: f64 = 20.0;

struct Driver {
    window_id: WindowId,
}

impl AppDriver for Driver {
    fn on_action(
        &mut self,
        window_id: WindowId,
        _ctx: &mut DriverCtx<'_, '_>,
        _widget_id: WidgetId,
        action: ErasedAction,
    ) {
        debug_assert_eq!(window_id, self.window_id, "unknown window");

        if action.is::<ButtonPress>() {
            println!("Hello");
        } else {
            // TODO: tracing::error?
            eprintln!("Unexpected action {action:?}");
        }
    }
}

fn main() {
    let label = Label::new("Hello")
        .with_style(StyleProperty::FontSize(32.0))
        // Ideally there's be an Into in Parley for this
        .with_style(StyleProperty::FontWeight(FontWeight::BOLD));
    let button = Button::new("Say hello");

    // Arrange the two widgets vertically, with some padding
    let main_widget = Flex::column()
        .with_child(label.with_auto_id())
        .with_spacer(VERTICAL_WIDGET_SPACING)
        .with_child(button.with_auto_id());

    let window_size = LogicalSize::new(400.0, 400.0);
    let window_attributes = Window::default_attributes()
        .with_title("Hello World!")
        .with_resizable(true)
        .with_min_inner_size(window_size);

    let driver = Driver {
        window_id: WindowId::next(),
    };

    masonry_winit::app::run(
        masonry_winit::app::EventLoop::with_user_event(),
        vec![(
            driver.window_id,
            window_attributes,
            NewWidget::new(main_widget).erased(),
        )],
        driver,
        default_property_set(),
    )
    .unwrap();
}
