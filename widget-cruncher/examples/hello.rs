
//! This is a very small example of how to setup a druid application.
//! It does the almost bare minimum while still being useful.

// On Windows platform, don't show a console when opening the app.
#![windows_subsystem = "windows"]

use widget_cruncher::widget::prelude::*;
use widget_cruncher::widget::{Flex, Label, Button};
use widget_cruncher::{AppLauncher, Data, Lens, UnitPoint, WindowDesc};

const VERTICAL_WIDGET_SPACING: f64 = 20.0;
const TEXT_BOX_WIDTH: f64 = 200.0;

pub fn main() {
    // describe the main window
    let main_window = WindowDesc::new(build_root_widget())
        .title("Hello World!")
        .window_size((400.0, 400.0));

    // start the application. Here we pass in the application state.
    AppLauncher::with_window(main_window)
        .log_to_console()
        .launch(())
        .expect("Failed to launch application");
}

fn build_root_widget() -> impl Widget<()> {
    let label = Label::new(|data: &(), _env: &Env| {
        "Hello".to_string()
    })
    .with_text_size(32.0);

    // a button that says "hello"
    let button = Button::new("Say hello");

    // arrange the two widgets vertically, with some padding
    Flex::column()
        .with_child(label)
        .with_spacer(VERTICAL_WIDGET_SPACING)
        .with_child(button)
}
