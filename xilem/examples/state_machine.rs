// Copyright 2024 the Xilem Authors
// SPDX-License-Identifier: Apache-2.0

//! A state machine to detect whether the button was pressed an even or an odd number of times.

use masonry::properties::types::AsUnit;
use winit::error::EventLoopError;
use xilem::core::one_of::{OneOf, OneOf3};
use xilem::style::Style as _;
use xilem::view::{button, flex, label, prose, sized_box, spinner};
use xilem::{EventLoop, WidgetView, WindowOptions, Xilem};

/// The state of the entire application.
///
/// This is owned by Xilem, used to construct the view tree, and updated by event handlers.
struct StateMachine {
    state: IsEven,
    /// The history of which transitions were taken in this run.
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

fn state_machine(app_data: &mut StateMachine) -> impl WidgetView<StateMachine> + use<> {
    match app_data.state {
        // The first time we use `OneOf` in a conditional statement, we need
        // to specify the number of `OneOf` variants used - 3 in this case.
        // This works around a rustc inference issue.
        IsEven::Initial | IsEven::Even => OneOf3::A(flex((
            sequence_button("1", IsEven::Odd),
            sequence_button("_", IsEven::Success),
        ))),
        // Subsequent branches can instead use the overarching `OneOf` type,
        // meaning that they don't need to change if additional branches are added.
        IsEven::Odd => OneOf::B(flex((
            sequence_button("1", IsEven::Even),
            sequence_button("_", IsEven::Halt),
        ))),
        // These branches can use the same variant of `OneOf`, because
        // they both have the same view type (`Label`).
        IsEven::Halt => OneOf::C(label("Failure! Tally total was odd.")),
        IsEven::Success => OneOf::C(label("Success! Tally total was even.")),
    }
}

/// A button component which transitions to a specified `target_state`
/// and appends its value to the history when pressed.
fn sequence_button(value: &'static str, target_state: IsEven) -> impl WidgetView<StateMachine> {
    button(value, move |app_data: &mut StateMachine| {
        app_data.state = target_state;
        app_data.history.push_str(value);
    })
}

fn app_logic(app_data: &mut StateMachine) -> impl WidgetView<StateMachine> + use<> {
    flex((
        button("Reset", |app_data: &mut StateMachine| {
            app_data.history.clear();
            app_data.state = IsEven::Initial;
        }),
        prose(&*app_data.history),
        label(format!("Current state: {:?}", app_data.state)),
        // TODO: Make `spinner` not need a `sized_box` to appear.
        sized_box(spinner()).height(40.px()).width(40.px()),
        state_machine(app_data),
        // TODO: When we have a canvas widget, visualise the entire state machine here.
    ))
    .padding(15.0)
}

fn main() -> Result<(), EventLoopError> {
    let app_data = StateMachine {
        state: IsEven::Initial,
        history: String::new(),
    };
    let app = Xilem::new_simple(app_data, app_logic, WindowOptions::new("Centered Flex"));
    app.run_in(EventLoop::builder())?;
    Ok(())
}
