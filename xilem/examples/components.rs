// Copyright 2024 the Xilem Authors
// SPDX-License-Identifier: Apache-2.0

//! Modularizing state can be done with `lens` which allows using modular components.

use winit::error::EventLoopError;
use xilem::core::lens;
use xilem::view::{MainAxisAlignment, button, flex, flex_row, label};
use xilem::{EventLoop, WidgetView, WindowOptions, Xilem};

#[derive(Default)]
struct AppState {
    modularized_count: i32,
    global_count: i32,
}

fn modular_counter(count: &mut i32) -> impl WidgetView<i32> + use<> {
    flex((
        label(format!("modularized count: {count}")),
        button("+", |count| *count += 1),
        button("-", |count| *count -= 1),
    ))
}

fn app_logic(state: &mut AppState) -> impl WidgetView<AppState> + use<> {
    flex_row((
        lens(modular_counter, |state: &mut AppState| {
            &mut state.modularized_count
        }),
        button(
            format!("clicked {} times", state.global_count),
            |state: &mut AppState| state.global_count += 1,
        ),
    ))
    .main_axis_alignment(MainAxisAlignment::Center)
}

fn main() -> Result<(), EventLoopError> {
    let app = Xilem::new_simple(
        AppState::default(),
        app_logic,
        WindowOptions::new("Components"),
    );
    app.run_in(EventLoop::builder())?;
    Ok(())
}
