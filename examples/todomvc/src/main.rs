use xilem::view::{button, h_stack, v_stack};
use xilem::{view::View, App, AppLauncher};

mod state;

use state::{AppState, Filter, Todo};

fn app_logic(data: &mut AppState) -> impl View<AppState> {
    println!("{data:?}");
    // The actual UI Code starts here
    v_stack((
        format!("There are {} todos", data.todos.len()),
        h_stack((
            button("All", |state: &mut AppState| state.filter = Filter::All),
            button("Active", |state: &mut AppState| {
                state.filter = Filter::Active
            }),
            button("Completed", |state: &mut AppState| {
                state.filter = Filter::Completed
            }),
        )),
    ))
    .with_spacing(20.0)
}

fn main() {
    let app = App::new(AppState::default(), app_logic);
    AppLauncher::new(app).run()
}
