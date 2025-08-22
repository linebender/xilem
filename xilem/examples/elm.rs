// Copyright 2024 the Xilem Authors
// SPDX-License-Identifier: Apache-2.0

//! Xilem supports several patterns for creating modular components.
//! You can also emulate the elm architecture for a subset of your app.
//! Though usually it's more idiomatic to modularize state with `map_state` and update state directly within event callbacks, as seen in the `components` example.

use masonry::properties::types::{CrossAxisAlignment, MainAxisAlignment};
use xilem::core::{MessageResult, map_action};
use xilem::view::{button, flex, flex_row, label};
use xilem::winit::dpi::LogicalSize;
use xilem::winit::error::EventLoopError;
use xilem::{EventLoop, WidgetView, WindowOptions, Xilem};
use xilem_core::{lens, map_message};

#[derive(Default)]
struct AppState {
    map_message_count: i32,
    map_action_count: i32,
}

enum CountMessage {
    Increment,
    Decrement,
}

// `map_action()` is basically how elm works, i.e. provide a message that the parent view has to handle to update the state.
// In this case the parent adjusts the count that is given to this view according to the message
fn elm_counter<T: 'static>(count: i32) -> impl WidgetView<T, CountMessage> {
    flex((
        label(format!("elm count: {count}")),
        button("+", |_| CountMessage::Increment),
        button("-", |_| CountMessage::Decrement),
    ))
}

/// A View Action type recording how the counter changed in [`map_message_counter`].
enum CounterChanged {
    Changed,
    Reset,
    Nop,
}

// `map_message` is the most flexible but also most verbose way to modularize the views by action.
// It's very similar to `map_action`, but it also allows to change the `MessageResult` for the parent view
fn map_message_counter(count: i32) -> impl WidgetView<i32, CounterChanged> {
    flex_row((
        flex((
            label(format!("map_message count: {count}")),
            button("+", |count| {
                *count += 1;
                CounterChanged::Changed
            }),
            button("-", |count| {
                *count -= 1;
                CounterChanged::Changed
            }),
        )),
        flex((
            button("reset all", |_| CounterChanged::Reset),
            button("do nothing (and don't rebuild the view tree)", |_| {
                CounterChanged::Nop
            }),
        )),
    ))
}

fn app_logic(state: &mut AppState) -> impl WidgetView<AppState> + use<> {
    flex_row((
        map_action(
            elm_counter(state.map_action_count),
            |state: &mut AppState, message| match message {
                CountMessage::Increment => state.map_action_count += 1,
                CountMessage::Decrement => state.map_action_count -= 1,
            },
        ),
        map_message(
            lens(
                |count| map_message_counter(*count),
                |state: &mut AppState| &mut state.map_message_count,
            ),
            |state: &mut AppState, message| {
                match message {
                    MessageResult::Action(CounterChanged::Reset) => {
                        state.map_message_count = 0;
                        state.map_action_count = 0;
                        MessageResult::Action(())
                    }
                    MessageResult::Action(CounterChanged::Nop) => MessageResult::Nop, // nothing changed, don't rebuild view tree
                    message_result => message_result.map(|_| ()), // just convert the result to `MessageResult<()>`
                }
            },
        ),
    ))
    .cross_axis_alignment(CrossAxisAlignment::Center)
    .main_axis_alignment(MainAxisAlignment::Center)
}

fn main() -> Result<(), EventLoopError> {
    let app = Xilem::new_simple(
        AppState::default(),
        app_logic,
        WindowOptions::new("Elm").with_min_inner_size(LogicalSize::new(600., 800.)),
    );
    app.run_in(EventLoop::builder())?;
    Ok(())
}
