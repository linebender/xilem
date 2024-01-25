use std::sync::Arc;
use xilem::view::{button, memoize, v_stack, AnyView};
use xilem::{view::View, App, AppLauncher};

// There are currently two ways to do memoization

fn app_logic(state: &mut AppState) -> impl View<AppState> {
    // The following is an example to do memoization with an Arc
    let increase_button = if let Some(view) = &state.count_view {
        view.clone()
    } else {
        let view = state.make_increase_button();
        state.count_view = Some(view.clone());
        view
    };

    v_stack((
        increase_button,
        // This is the alternative with Memoize
        // Note how this requires a closure that returns the memoized view, while Arc does not
        memoize(state.count, |count| {
            button(
                format!("decrease the count: {count}"),
                |data: &mut AppState| {
                    data.count_view = None;
                    data.count -= 1;
                },
            )
        }),
        button("reset", |data: &mut AppState| {
            if data.count != 0 {
                data.count_view = None;
            }
            data.count = 0;
        }),
    ))
    .with_spacing(20.0)
}

struct AppState {
    count: i32,
    count_view: Option<Arc<dyn AnyView<AppState>>>,
}

impl AppState {
    fn make_increase_button(&self) -> Arc<dyn AnyView<AppState>> {
        Arc::new(button(
            format!("current count is {}", self.count),
            |state: &mut AppState| {
                state.count += 1;
                state.count_view = None;
            },
        ))
    }
}

fn main() {
    let data = AppState {
        count: 0,
        count_view: None,
    };

    AppLauncher::new(App::new(data, app_logic)).run()
}
