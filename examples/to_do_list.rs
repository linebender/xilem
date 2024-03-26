// This software is licensed under Apache License 2.0 and distributed on an
// "as-is" basis without warranties of any kind. See the LICENSE file for
// details.

#![windows_subsystem = "windows"]

use masonry::widget::{
    Button, CrossAxisAlignment, Flex, Label, Portal, SizedBox, TextBox, WidgetMut,
};
use masonry::{
    Action, AppDelegate, AppLauncher, Color, DelegateCtx, WidgetId, WindowDescription, WindowId,
};

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
    ) {
        match action {
            Action::ButtonPressed | Action::TextEntered(_) => {
                let mut root: WidgetMut<Portal<Flex>> = ctx.get_root();
                if !self.next_task.is_empty() {
                    let mut flex = root.child_mut();
                    flex.child_mut(2)
                        .unwrap()
                        .downcast::<SizedBox>()
                        .unwrap()
                        .child_mut()
                        .unwrap()
                        .downcast::<Flex>()
                        .unwrap()
                        .add_child(Label::new(self.next_task.clone()));
                }
            }
            Action::TextChanged(new_text) => {
                self.next_task = new_text;
            }
            _ => {}
        }
    }
}

fn main() {
    const GAP_SIZE: f64 = 4.0;
    const LIGHT_GRAY: Color = Color::rgb8(0x71, 0x71, 0x71);
    // The main button with some space below, all inside a scrollable area.
    let root_widget = Portal::new(
        Flex::column()
            .with_child(
                SizedBox::new(
                    Flex::row()
                        .with_child(Button::new("Add task"))
                        .with_spacer(5.0)
                        .with_flex_child(TextBox::new(""), 1.0),
                )
                .border(LIGHT_GRAY, GAP_SIZE),
            )
            .with_spacer(GAP_SIZE)
            .with_child(
                SizedBox::new(
                    Flex::column()
                        .cross_axis_alignment(CrossAxisAlignment::Start)
                        .with_child(Label::new("List items:")),
                )
                .expand_width()
                .border(LIGHT_GRAY, GAP_SIZE),
            ),
    )
    .constrain_horizontal(true);

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
