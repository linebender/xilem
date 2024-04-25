// On Windows platform, don't show a console when opening the app.
#![windows_subsystem = "windows"]
use masonry::ArcStr;
use xilem_masonry::view::{button, flex};
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

    let mut sequence = vec![
        the_button(label, 1),
        the_button("Decrement".to_string(), -1),
    ];
    for x in 0..count {
        sequence.push(the_button(format!("+{}", x), x));
    }
    flex(sequence)
}

fn the_button(label: impl Into<ArcStr>, count: i32) -> impl MasonryView<AppData> {
    button(label, move |data: &mut AppData| {
        println!("clicked");
        data.count += count;
    })
}

struct AppData {
    count: i32,
}

fn main() {
    let data = AppData { count: 0 };

    let app = Xilem::new(data, app_logic);
    app.run_windowed("First Example".into()).unwrap();
}
