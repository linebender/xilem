// Copyright 2024 the Xilem Authors
// SPDX-License-Identifier: Apache-2.0

use std::sync::Arc;
use xilem::view::{button, flex, memoize};
use xilem::{AnyWidgetView, EventLoop, WidgetView, Xilem};

// There are currently two ways to do memoization

struct AppState {
    count: i32,
    increase_button: MemoizedArcView<i32>,
}

#[derive(Default)]
struct MemoizedArcView<D> {
    data: D,
    // When TAITs are stabilized this can be a non-erased concrete type
    view: Option<Arc<AnyWidgetView<AppState>>>,
}

// The following is an example to do memoization with an Arc
fn increase_button(state: &mut AppState) -> Arc<AnyWidgetView<AppState>> {
    if state.count != state.increase_button.data || state.increase_button.view.is_none() {
        let view = Arc::new(button(
            format!("current count is {}", state.count),
            |state: &mut AppState| {
                state.count += 1;
            },
        ));
        state.increase_button.data = state.count;
        state.increase_button.view = Some(view.clone());
        view
    } else {
        state.increase_button.view.as_ref().unwrap().clone()
    }
}

// This is the alternative with Memoize
// Note how this requires a closure that returns the memoized view, while Arc does not
fn decrease_button(state: &AppState) -> impl WidgetView<AppState> {
    memoize(state.count, |count| {
        button(
            format!("decrease the count: {count}"),
            |data: &mut AppState| data.count -= 1,
        )
    })
}

fn reset_button() -> impl WidgetView<AppState> {
    button("reset", |data: &mut AppState| data.count = 0)
}

fn app_logic(state: &mut AppState) -> impl WidgetView<AppState> {
    flex((
        increase_button(state),
        decrease_button(state),
        reset_button(),
    ))
}

fn main() {
    let data = AppState {
        count: 0,
        increase_button: Default::default(),
    };

    let app = Xilem::new(data, app_logic);
    app.run_windowed(EventLoop::with_user_event(), "Memoization".into())
        .unwrap();
}
