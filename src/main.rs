use glazier::kurbo::Circle;
use vello::peniko::Color;
use xilem::vg::{color, group, vg};
use xilem::view::{button, h_stack, v_stack};
use xilem::{view::View, App, AppLauncher};

#[derive(Default)]
struct GraphicsState {
    circles: Vec<Circle>,
    current: usize,
}

fn graphics_app(state: &mut GraphicsState) -> impl View<GraphicsState> {
    let rendered = state
        .circles
        .iter()
        .enumerate()
        .map(|(i, circle)| {
            let c = if i == state.current {
                Color::RED
            } else {
                Color::LIGHT_GRAY
            };
            color(*circle, c)
        })
        .collect::<Vec<_>>();
    v_stack((
        h_stack((
            button("Add circle", |state: &mut GraphicsState| {
                let x = 30.0 + 50.0 * state.circles.len() as f64;
                state.circles.push(Circle::new((x, 100.0), 22.0));
            }),
            button("Move right", |state: &mut GraphicsState| state.current += 1),
            button("Move left", |state: &mut GraphicsState| {
                state.current = state.current.saturating_sub(1)
            }),
        )),
        vg(group(rendered)),
    ))
}

fn app_logic(data: &mut i32) -> impl View<i32> {
    // here's some logic, deriving state for the view from our state
    let label = if *data == 1 {
        "clicked 1 time".to_string()
    } else {
        format!("clicked {data} times")
    };

    // The actual UI Code starts here
    v_stack((
        button(label, |data| {
            println!("clicked");
            *data += 1;
        }),
        vg(group((
            Circle::new((200.0, 200.0), 100.0),
            color(Circle::new((350.0, 200.0), 20.0), Color::RED),
        ))),
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
    //let app = App::new(0, app_logic);
    let app = App::new(Default::default(), graphics_app);
    AppLauncher::new(app).run()
}
