// Copyright 2024 the Xilem Authors
// SPDX-License-Identifier: Apache-2.0

//! Displaying a variable length list is achieved using a [Vec] `FlexSequence`.

use winit::error::EventLoopError;
use xilem::view::{Axis, MainAxisAlignment, button, flex, prose};
use xilem::{EventLoop, WidgetView, WindowOptions, Xilem};

#[derive(Default)]
struct AppState {
    count: usize,
}

fn app_logic(state: &mut AppState) -> impl WidgetView<AppState> + use<> {
    // A vector (Vec) of views can be used as part of a `flex`'s children, allowing collections of dynamic length to be displayed.
    let list = (0..state.count)
        .map(|n| prose(format!("item #{n}")))
        .collect::<Vec<_>>();

    flex((
        // Even when a `Vec` is used for the children, other widgets can be included by putting them in a
        // tuple of children alongside the vector.
        button("more", |appstate: &mut AppState| appstate.count += 1),
        list,
    ))
    .direction(Axis::Vertical) // Top to Bottom
    // We can control alignment of the elements in the flexbox
    .main_axis_alignment(MainAxisAlignment::Start) // Aligned to the left
}

fn main() -> Result<(), EventLoopError> {
    let app = Xilem::new_simple(AppState::default(), app_logic, WindowOptions::new("Lists"));
    app.run_in(EventLoop::builder())?;
    Ok(())
}
