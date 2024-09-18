// Copyright 2024 the Xilem Authors
// SPDX-License-Identifier: Apache-2.0

//! Modularizing state can be done with `lens` which allows using modular components.

use masonry::widget::MainAxisAlignment;
use winit::error::EventLoopError;
use xilem::{
    core::lens,
    view::{button, flex, label, Axis},
    EventLoop, WidgetView, Xilem,
};

#[derive(Default)]
struct AppState {
    modularized_count: i32,
    global_count: i32,
}

fn modular_counter(count: &mut i32) -> impl WidgetView<i32> {
    flex((
        label(format!("modularized count: {count}")),
        button("+", |count| *count += 1),
        button("-", |count| *count -= 1),
    ))
}

fn app_logic(state: &mut AppState) -> impl WidgetView<AppState> {
    flex((
        lens(modular_counter, state, |state| &mut state.modularized_count),
        button(
            format!("clicked {} times", state.global_count),
            |state: &mut AppState| state.global_count += 1,
        ),
    ))
    .direction(Axis::Horizontal)
    .main_axis_alignment(MainAxisAlignment::Center)
}

fn main() -> Result<(), EventLoopError> {
    let app = Xilem::new(AppState::default(), app_logic);
    app.run_windowed(EventLoop::with_user_event(), "Components".into())?;
    Ok(())
}
