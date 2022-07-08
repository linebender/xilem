//! This is a very small example of how to setup a druid application.
//! It does the almost bare minimum while still being useful.

// On Windows platform, don't show a console when opening the app.
#![windows_subsystem = "windows"]

// TODO - rework imports
use widget_cruncher::action::Action;
use widget_cruncher::app_delegate::{AppDelegate, DelegateCtx};

use widget_cruncher::widget::{prelude::*, TextBox};
use widget_cruncher::widget::{Button, Flex};
use widget_cruncher::{AppLauncher, WindowDesc, WindowId};

const VERTICAL_WIDGET_SPACING: f64 = 20.0;

struct Delegate;

impl AppDelegate for Delegate {
    fn on_action(
        &mut self,
        _ctx: &mut DelegateCtx,
        _window_id: WindowId,
        _widget_id: WidgetId,
        action: Action,
        _env: &Env,
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
    let main_window = WindowDesc::new(build_root_widget())
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
    let label = TextBox::new();

    // a button that says "hello"
    let button = Button::new("Say hello");

    // arrange the two widgets vertically, with some padding
    Flex::column()
        .with_child(label)
        .with_spacer(VERTICAL_WIDGET_SPACING)
        .with_child(button)
}
