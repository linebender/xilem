use xilem::{App, AppLauncher, View};

fn app_logic(_data: &mut ()) -> impl View<()> {
    "hello world".to_string()
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
    let app = App::new((), app_logic);
    AppLauncher::new(app).run()
}
