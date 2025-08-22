// Copyright 2024 the Xilem Authors
// SPDX-License-Identifier: Apache-2.0

//! You can use memoization to avoid allocations.

use std::sync::Arc;

use xilem::core::{frozen, memoize};
use xilem::view::{button, flex};
use xilem::{AnyWidgetView, EventLoop, WidgetView, WindowOptions, Xilem};

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
fn decrease_button(state: &AppState) -> impl WidgetView<AppState> + use<> {
    memoize(state.count, |count| {
        button(
            format!("decrease the count: {count}"),
            |data: &mut AppState| data.count -= 1,
        )
    })
}

fn reset_button() -> impl WidgetView<AppState> {
    // The contents of this view never changes, so we use `frozen` to avoid unnecessary rebuilds.
    // This is a special case of memoization for when the view doesn't depend on any data.
    frozen(|| button("reset", |data: &mut AppState| data.count = 0))
}

fn app_logic(state: &mut AppState) -> impl WidgetView<AppState> + use<> {
    flex((
        increase_button(state),
        decrease_button(state),
        reset_button(),
    ))
}

fn main() {
    let data = AppState {
        count: 0,
        increase_button: MemoizedArcView::default(),
    };

    let app = Xilem::new_simple(data, app_logic, WindowOptions::new("Memoization"));
    app.run_in(EventLoop::builder()).unwrap();
}
