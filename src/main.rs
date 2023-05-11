use xilem::view::{
    button, fixed, flex, flex_spacer, h_stack, sizeable, spacer, v_flex, v_stack, MainAxisAlignment,
};
use xilem::{view::View, App, AppLauncher};

fn app_logic(data: &mut i32) -> impl View<i32> {
    // here's some logic, deriving state for the view from our state
    let label = if *data == 1 {
        "clicked 1 time".to_string()
    } else {
        format!("clicked {data} times")
    };

    // The actual UI Code starts here
    v_flex((
        // flex(
        //     sizeable(button(label, |data| {
        //         println!("clicked");
        //         *data += 1;
        //     }))
        //     .expand(),
        //     1.0,
        // ),
        fixed(button(label.clone(), |data| {
            println!("clicked");
            *data += 1;
        })),
        fixed(h_stack((
            button("decrease", |data| {
                println!("clicked decrease");
                *data -= 1;
            }),
            button("reset", |data| {
                println!("clicked reset");
                *data = 0;
            }),
        ))),
    ))
    .must_fill_main_axis(true)
    .main_axis_alignment(MainAxisAlignment::SpaceEvenly)
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

    use tracing_subscriber::prelude::*;
    let target_filter =
        tracing_subscriber::filter::Targets::new().with_target("xilem", tracing::Level::TRACE);
    // let level_filter = tracing_subscriber::filter::LevelFilter::DEBUG;
    let fmt_layer = tracing_subscriber::fmt::layer()
        // Display target (eg "my_crate::some_mod::submod") with logs
        .with_target(true);

    tracing_subscriber::registry()
        // .with(level_filter)
        .with(target_filter)
        .with(fmt_layer)
        .init();

    let app = App::new(0, app_logic);
    AppLauncher::new(app).run()
}
