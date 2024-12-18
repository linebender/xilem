// Copyright 2024 the Xilem Authors
// SPDX-License-Identifier: Apache-2.0

//! Xilem supports several patterns for creating modular components.
//! You can also emulate the elm architecture for a subset of your app.
//! Though usually it's more idiomatic to modularize state with `map_state` and update state directly within event callbacks, as seen in the `components` example.

#![expect(clippy::shadow_unrelated, reason = "Idiomatic for Xilem users")]

use masonry::widget::{CrossAxisAlignment, MainAxisAlignment};
use winit::error::EventLoopError;
use xilem::core::{adapt, map_action, MessageResult};
use xilem::view::{button, flex, label, Axis};
use xilem::{EventLoop, WidgetView, Xilem};

#[derive(Default)]
struct AppState {
    adapt_count: i32,
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

enum AdaptMessage {
    Changed,
    Reset,
    Nop,
}

// `adapt()` is the most flexible but also most verbose way to modularize the views by state and action,
// This is basically a combination of `map_state` and `map_action`, but it also allows to change the `MessageResult` for the parent view
fn adapt_counter(count: i32) -> impl WidgetView<i32, AdaptMessage> {
    flex((
        flex((
            label(format!("adapt count: {count}")),
            button("+", |count| {
                *count += 1;
                AdaptMessage::Changed
            }),
            button("-", |count| {
                *count -= 1;
                AdaptMessage::Changed
            }),
        )),
        flex((
            button("reset all", |_| AdaptMessage::Reset),
            button("do nothing (and don't rebuild the view tree)", |_| {
                AdaptMessage::Nop
            }),
        )),
    ))
    .direction(Axis::Horizontal)
}

fn app_logic(state: &mut AppState) -> impl WidgetView<AppState> {
    flex((
        map_action(
            elm_counter(state.map_action_count),
            |state: &mut AppState, message| match message {
                CountMessage::Increment => state.map_action_count += 1,
                CountMessage::Decrement => state.map_action_count -= 1,
            },
        ),
        adapt(
            adapt_counter(state.adapt_count),
            |state: &mut AppState, thunk| match thunk.call(&mut state.adapt_count) {
                MessageResult::Action(AdaptMessage::Reset) => {
                    state.adapt_count = 0;
                    state.map_action_count = 0;
                    MessageResult::Action(())
                }
                MessageResult::Action(AdaptMessage::Nop) => MessageResult::Nop, // nothing changed, don't rebuild view tree
                message_result => message_result.map(|_| ()), // just convert the result to `MessageResult<()>`
            },
        ),
    ))
    .direction(Axis::Horizontal)
    .cross_axis_alignment(CrossAxisAlignment::Center)
    .main_axis_alignment(MainAxisAlignment::Center)
}

fn main() -> Result<(), EventLoopError> {
    let app = Xilem::new(AppState::default(), app_logic);
    app.run_windowed(EventLoop::with_user_event(), "Elm".into())?;
    Ok(())
}
