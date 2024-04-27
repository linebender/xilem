// On Windows platform, don't show a console when opening the app.
#![windows_subsystem = "windows"]

use xilem_masonry::view::{button, checkbox, flex, label};
use xilem_masonry::{Axis, BoxedMasonryView, Color, MasonryView, TextAlignment, Xilem};

fn app_logic(data: &mut AppData) -> impl MasonryView<AppData> {
    // here's some logic, deriving state for the view from our state
    let count = data.count;
    let button_label = if count == 1 {
        "clicked 1 time".to_string()
    } else {
        format!("clicked {count} times")
    };

    // The actual UI Code starts here

    let sequence = (0..count)
        .map(|x| button(format!("+{x}"), move |data: &mut AppData| data.count += x))
        .collect::<Vec<_>>();
    flex((
        flex((
            label("Label")
                .color(Color::REBECCA_PURPLE)
                .alignment(TextAlignment::Start),
            label("Disabled label").disabled(),
        ))
        .direction(Axis::Horizontal),
        button(button_label, |data: &mut AppData| data.count += 1),
        checkbox("Check me", data.active, |data: &mut AppData, checked| {
            data.active = checked;
        }),
        toggleable(data),
        button("Decrement", |data: &mut AppData| data.count -= 1),
        button("Reset", |data: &mut AppData| data.count = 0),
        sequence,
    ))
}

fn toggleable(data: &mut AppData) -> impl MasonryView<AppData> {
    let inner_view: BoxedMasonryView<_, _> = if data.active {
        Box::new(flex((
            button("Deactivate", |data: &mut AppData| {
                data.active = false;
            }),
            button("Unlimited Power", |data: &mut AppData| {
                data.count = -1_000_000;
            }),
        )))
    } else {
        Box::new(button("Activate", |data: &mut AppData| data.active = true))
    };
    inner_view
}

struct AppData {
    count: i32,
    active: bool,
}

fn main() {
    let data = AppData {
        count: 0,
        active: false,
    };

    let app = Xilem::new(data, app_logic);
    app.run_windowed("First Example".into()).unwrap();
}
