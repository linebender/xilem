use std::sync::Arc;
use xilem::view::{button, flex, memoize};
use xilem::{AnyMasonryView, MasonryView, Xilem};

// There are currently two ways to do memoization

fn app_logic(state: &mut AppState) -> impl MasonryView<AppState> {
    // The following is an example to do memoization with an Arc
    let increase_button = if let Some(view) = &state.count_view {
        view.clone()
    } else {
        let view = state.make_increase_button();
        state.count_view = Some(view.clone());
        view
    };

    flex((
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
}

struct AppState {
    count: i32,
    // When TAITs are stabilized this can be a non-erased concrete type
    count_view: Option<Arc<dyn AnyMasonryView<AppState>>>,
}

impl AppState {
    fn make_increase_button(&self) -> Arc<dyn AnyMasonryView<AppState>> {
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

    let app = Xilem::new(data, app_logic);
    app.run_windowed("Memoization".into()).unwrap();
}
