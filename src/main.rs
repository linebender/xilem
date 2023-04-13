use xilem::view::{AnyView, button, h_stack, v_stack, View};
use xilem::{App, AppLauncher};
use xilem::widget::Pod;

fn app_logic(data: &mut i32) -> impl View<Pod, i32> {
    // here's some logic, deriving state for the view from our state
    let label = if *data == 1 {
        "clicked 1 time".to_string()
    } else {
        format!("clicked {data} times")
    };

    // The actual UI Code starts here

    let view = match *data {
        0 => Box::new(button(label, |data| {
            println!("clicked");
            *data += 1;
        })) as Box<dyn AnyView<Pod, i32> + Send>,
        _ => Box::new(v_stack((
            button(label, |data| {
                println!("clicked");
                *data += 1;
            }),
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
        )).with_spacing(20.0)) as Box<dyn AnyView<Pod, i32> + Send>,
    };
    view
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
