// Copyright 2024 the Xilem Authors
// SPDX-License-Identifier: Apache-2.0

//! A state machine to detect whether the button was pressed an even or an odd number of times.

use winit::error::EventLoopError;
use xilem::{
    view::{button, flex, label, prose},
    EventLoop, WidgetView, Xilem,
};
use xilem_core::one_of::{OneOf4, OneOf9};

struct AppState {
    value: IsEven,
    history: String,
}

#[derive(Copy, Clone, Debug)]
enum IsEven {
    Initial,
    Odd,
    Even,
    Halt,
    Success,
}

fn sequence_button(value: &'static str, target_state: IsEven) -> impl WidgetView<AppState> {
    button(value, move |state: &mut AppState| {
        state.value = target_state;
        state.history.push_str(value);
    })
}

fn state_machine(state: &mut AppState) -> impl WidgetView<AppState> {
    match state.value {
        IsEven::Initial | IsEven::Even => OneOf4::A(flex((
            sequence_button("1", IsEven::Odd),
            sequence_button("_", IsEven::Success),
        ))),
        IsEven::Odd => OneOf9::D(flex((
            sequence_button("1", IsEven::Even),
            sequence_button("_", IsEven::Halt),
        ))),
        IsEven::Halt => OneOf9::B(label("Failure! Tally total was odd.")),
        IsEven::Success => OneOf9::C(label("Success! Tally total was even.")),
    }
}

fn app_logic(state: &mut AppState) -> impl WidgetView<AppState> {
    flex((
        button("Reset", |state: &mut AppState| {
            state.history.clear();
            state.value = IsEven::Initial;
        }),
        prose(&*state.history),
        label(format!("Current state: {:?}", state.value)),
        state_machine(state),
    ))
}

fn main() -> Result<(), EventLoopError> {
    let state = AppState {
        value: IsEven::Initial,
        history: String::new(),
    };
    let app = Xilem::new(state, app_logic);
    app.run_windowed(EventLoop::with_user_event(), "Centered Flex".into())?;
    Ok(())
}
