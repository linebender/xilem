// This software is licensed under Apache License 2.0 and distributed on an
// "as-is" basis without warranties of any kind. See the LICENSE file for
// details.

#![windows_subsystem = "windows"]

use masonry::widget::{prelude::*, TextBox};
use masonry::widget::{Button, Flex, Label, Portal, WidgetMut};
use masonry::Action;
use masonry::{AppDelegate, AppLauncher, DelegateCtx, WindowDescription, WindowId};

const VERTICAL_WIDGET_SPACING: f64 = 20.0;

struct Delegate {
    next_task: String,
}

impl AppDelegate for Delegate {
    fn on_action(
        &mut self,
        ctx: &mut DelegateCtx,
        _window_id: WindowId,
        _widget_id: WidgetId,
        action: Action,
        _env: &Env,
    ) {
        match action {
            Action::ButtonPressed => {
                let mut root: WidgetMut<Portal<Flex>> = ctx.get_root();
                let mut flex = root.child_mut();
                flex.add_child(Label::new(self.next_task.clone()));
            }
            Action::TextChanged(new_text) => {
                self.next_task = new_text.clone();
            }
            _ => {}
        }
    }
}

fn main() {
    // The main button with some space below, all inside a scrollable area.
    let root_widget = Portal::new(
        Flex::column()
            .with_child(
                Flex::row()
                    .with_child(TextBox::new(""))
                    .with_child(Button::new("Add task")),
            )
            .with_spacer(VERTICAL_WIDGET_SPACING),
    );

    let main_window = WindowDescription::new(root_widget)
        .title("To-do list")
        .window_size((400.0, 400.0));

    AppLauncher::with_window(main_window)
        .with_delegate(Delegate {
            next_task: String::new(),
        })
        .log_to_console()
        .launch()
        .expect("Failed to launch application");
}
