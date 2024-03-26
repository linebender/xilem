// This software is licensed under Apache License 2.0 and distributed on an
// "as-is" basis without warranties of any kind. See the LICENSE file for
// details.

//! This is a very small example of how to setup a masonry application.
//! It does the almost bare minimum while still being useful.

// On Windows platform, don't show a console when opening the app.
#![windows_subsystem = "windows"]

use masonry::widget::prelude::*;
use masonry::widget::{Button, Flex, Label};
use masonry::Action;
use masonry::{AppDelegate, DelegateCtx};
use masonry::{AppLauncher, WindowDescription, WindowId};

const VERTICAL_WIDGET_SPACING: f64 = 20.0;

struct Delegate;

impl AppDelegate for Delegate {
    fn on_action(
        &mut self,
        _ctx: &mut DelegateCtx,
        _window_id: WindowId,
        _widget_id: WidgetId,
        action: Action,
    ) {
        if let Action::ButtonPressed = action {
            println!("Hello");
        }
    }
}

pub fn main() {
    // describe the main window
    let main_window = WindowDescription::new(build_root_widget())
        .title("Hello World!")
        .window_size((400.0, 400.0));

    // start the application. Here we pass in the application state.
    AppLauncher::with_window(main_window)
        .with_delegate(Delegate)
        .log_to_console()
        .launch()
        .expect("Failed to launch application");
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
