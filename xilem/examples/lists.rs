// Copyright 2024 the Xilem Authors
// SPDX-License-Identifier: Apache-2.0

//! Displaying a variable length list of widgets is trivially done using any kind of FlexSequence

use winit::error::EventLoopError;
use xilem::view::{Axis, MainAxisAlignment, button, flex, prose};
use xilem::{EventLoop, WidgetView, WindowOptions, Xilem};

#[derive(Default)]
struct AppState {
    count: usize,
}

fn app_logic(state: &mut AppState) -> impl WidgetView<AppState> + use<> {
    // Here's the magic
    // Creating a list of widgets can be as easy as using a flex(some_vec)
    // with the alignment described using the flex(...) widget.
    let list = (0..state.count)
        .map(|n| prose(format!("item #{}", n)))
        .collect::<Vec<_>>();

    flex((
        button("more", |appstate: &mut AppState| appstate.count += 1),
        list,
    ))
    .direction(Axis::Vertical)
    .main_axis_alignment(MainAxisAlignment::Start)
}

fn main() -> Result<(), EventLoopError> {
    let app = Xilem::new_simple(AppState::default(), app_logic, WindowOptions::new("Lists"));
    app.run_in(EventLoop::with_user_event())?;
    Ok(())
}
