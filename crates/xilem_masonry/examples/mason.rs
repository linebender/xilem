// On Windows platform, don't show a console when opening the app.
#![windows_subsystem = "windows"]
use xilem_masonry::view::button;
use xilem_masonry::{MasonryView, Xilem};

fn app_logic(data: &mut AppData) -> impl MasonryView<AppData> {
    // here's some logic, deriving state for the view from our state
    let count = data.count;
    let label = if count == 1 {
        "clicked 1 time".to_string()
    } else {
        format!("clicked {count} times")
    };

    // The actual UI Code starts here

    button(label, |data: &mut AppData| {
        println!("clicked");
        data.count += 1;
    })
}

struct AppData {
    count: i32,
}

fn main() {
    let data = AppData { count: 0 };

    let app = Xilem::new(data, app_logic);
    app.run_windowed("First Example".into()).unwrap()
}
