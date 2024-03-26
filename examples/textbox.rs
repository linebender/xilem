// This software is licensed under Apache License 2.0 and distributed on an
// "as-is" basis without warranties of any kind. See the LICENSE file for
// details.

// On Windows platform, don't show a console when opening the app.
#![windows_subsystem = "windows"]

// TODO - rework imports - See #14
use masonry::widget::prelude::*;
use masonry::widget::{Button, Flex, TextBox};
use masonry::{Action, AppDelegate, AppLauncher, DelegateCtx, WindowDescription, WindowId};

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
        match action {
            Action::ButtonPressed => {
                // TODO - Print textbox contents
                println!("Hello");
            }
            _ => {}
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
    let label = TextBox::new("").with_placeholder("Some text");

    // a button that says "hello"
    let button = Button::new("Say hello");

    // arrange the two widgets vertically, with some padding
    Flex::column()
        .with_child(label)
        .with_spacer(VERTICAL_WIDGET_SPACING)
        .with_child(button)
}
