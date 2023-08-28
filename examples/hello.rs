use xilem::view::{button, h_stack, switch, v_stack};
use xilem::{view::View, App, AppLauncher};

fn app_logic(data: &mut AppData) -> impl View<AppData> {
    // here's some logic, deriving state for the view from our state
    let count = data.count;
    let label = if count == 1 {
        "clicked 1 time".to_string()
    } else {
        format!("clicked {count} times")
    };

    // The actual UI Code starts here
    v_stack((
        button(label, |data: &mut AppData| {
            println!("clicked");
            data.count += 1;
        }),
        h_stack((
            button("decrease", |data: &mut AppData| {
                println!("clicked decrease");
                data.count -= 1;
            }),
            button("reset", |data: &mut AppData| {
                println!("clicked reset");
                data.count = 0;
            }),
            switch(data.is_on, |data: &mut AppData, value: bool| {
                data.is_on = value
            }),
        )),
    ))
    .with_spacing(20.0)
}

struct AppData {
    count: i32,
    is_on: bool,
}

fn main() {
    /*
    let app = Application::new().unwrap();
    let mut window_builder = glazier::WindowBuilder::new(app.clone());
    window_builder.resizable(false);
    window_builder.set_size((WIDTH as f64 / 2., HEIGHT as f64 / 2.).into());
    window_builder.set_handler(Box::new(xilem::WindowState::new()));
    let window_handle = window_builder.build().unwrap();
    window_handle.show();
    app.run(None);
    */
    let data = AppData {
        count: 0,
        is_on: false,
    };

    let app = App::new(data, app_logic);
    AppLauncher::new(app).run()
}
