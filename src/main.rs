use xilem::view::{button, h_stack, v_stack, WidgetBound};
use xilem::{view::View, AnyView, App, AppLauncher};

fn app_logic(data: &mut i32) -> impl View<i32> {
    // here's some logic, deriving state for the view from our state
    let label = if *data == 1 {
        "clicked 1 time".to_string()
    } else {
        format!("clicked {data} times")
    };
    let widget: Box<dyn AnyView<i32, WidgetBound> + Send> = Box::new(button(label, |data| {
        println!("clicked");
        *data += 1;
    }));

    // The actual UI Code starts here
    v_stack((
        widget,
        h_stack((
            button("decrease", |data| {
                println!("clicked decrease");
                *data -= 1;
            }),
            button("reset", |data| {
                println!("clicked reset");
                *data = 0;
            }),
        )),
    ))
    .with_spacing(20.0)
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
    let app = App::new(0, app_logic);
    AppLauncher::new(app).run()
}
