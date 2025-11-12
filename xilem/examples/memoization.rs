// Copyright 2024 the Xilem Authors
// SPDX-License-Identifier: Apache-2.0

//! You can use memoization to avoid allocations.

use std::sync::Arc;

use xilem::core::{frozen, memoize};
use xilem::view::{flex_col, text_button};
use xilem::{AnyWidgetView, EventLoop, WidgetView, WindowOptions, Xilem};
use xilem_core::Edit;

// There are currently two ways to do memoization

struct AppState {
    count: i32,
    increase_button: MemoizedArcView<i32>,
}

#[derive(Default)]
struct MemoizedArcView<D> {
    data: D,
    // When TAITs are stabilized this can be a non-erased concrete type
    view: Option<Arc<AnyWidgetView<Edit<AppState>>>>,
}

// The following is an example to do memoization with an Arc
fn increase_button(state: &mut AppState) -> Arc<AnyWidgetView<Edit<AppState>>> {
    if state.count != state.increase_button.data || state.increase_button.view.is_none() {
        let view = Arc::new(text_button(
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
fn decrease_button(state: &AppState) -> impl WidgetView<Edit<AppState>> + use<> {
    memoize(state.count, |count| {
        // I think text_button needs rewriting to not run into https://github.com/rust-lang/rust/issues/117392
        Box::new(text_button(
            format!("decrease the count: {count}"),
            |data: &mut AppState| data.count -= 1,
        )) as Box<AnyWidgetView<Edit<AppState>>>
    })
}

fn reset_button() -> impl WidgetView<Edit<AppState>> {
    // The contents of this view never changes, so we use `frozen` to avoid unnecessary rebuilds.
    // This is a special case of memoization for when the view doesn't depend on any data.
    // TODO(DJMcNab): I think text_button needs rewriting to not run into https://github.com/rust-lang/rust/issues/117392
    frozen(|| {
        Box::new(text_button("reset", |data: &mut AppState| data.count = 0))
            as Box<AnyWidgetView<Edit<AppState>>>
    })
}

fn app_logic(state: &mut AppState) -> impl WidgetView<Edit<AppState>> + use<> {
    flex_col((
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
    app.run_in(EventLoop::with_user_event()).unwrap();
}
