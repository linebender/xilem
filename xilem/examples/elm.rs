// Copyright 2024 the Xilem Authors
// SPDX-License-Identifier: Apache-2.0

use masonry::widget::{CrossAxisAlignment, MainAxisAlignment};
use winit::error::EventLoopError;
use xilem::{
    view::{button, flex, label},
    EventLoop, WidgetView, Xilem,
};
use xilem_core::{adapt, map_action, map_state, MessageResult};

#[derive(Default)]
struct AppState {
    adapt_count: i32,
    map_state_count: i32,
    map_action_count: i32,
}

// `map_state()` maps a subset of the state from the parent, such that views can be modularized by state
fn map_state_view(count: &mut i32) -> impl WidgetView<i32> {
    flex((
        label(format!("adapt state count: {}", count)),
        button("+", |count| *count += 1),
        button("-", |count| *count -= 1),
    ))
}

enum CountMessage {
    Increment,
    Decrement,
}

// `map_action()` is basically how elm works, i.e. provide a message that the parent view has to handle to update the state.
// In this case the parent adjusts the count that is given to this view according to the message
fn map_action_view<T>(count: i32) -> impl WidgetView<T, CountMessage> {
    flex((
        label(format!("adapt state count: {}", count)),
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
// This is basically a combination of the two ways above, but it also allows to change the `MessageResult` for the parent view
fn adapt_view(count: i32) -> impl WidgetView<i32, AdaptMessage> {
    flex((
        flex((
            label(format!("adapt count: {}", count)),
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
    .direction(xilem::Axis::Horizontal)
}

fn app_logic(state: &mut AppState) -> impl WidgetView<AppState> {
    flex((
        map_state(
            map_state_view(&mut state.map_state_count),
            |state: &mut AppState| &mut state.map_state_count,
        ),
        map_action(
            map_action_view(state.map_action_count),
            |state: &mut AppState, message| match message {
                CountMessage::Increment => state.map_action_count += 1,
                CountMessage::Decrement => state.map_action_count -= 1,
            },
        ),
        adapt(
            adapt_view(state.adapt_count),
            |state: &mut AppState, thunk| match thunk.call(&mut state.adapt_count) {
                MessageResult::Action(AdaptMessage::Reset) => {
                    state.adapt_count = 0;
                    state.map_state_count = 0;
                    state.map_action_count = 0;
                    MessageResult::Action(())
                }
                MessageResult::Action(AdaptMessage::Nop) => MessageResult::Nop, // nothing changed, don't rebuild view tree
                message_result => message_result.map(|_| ()), // just convert the result to `MessageResult<()>`
            },
        ),
    ))
    .direction(xilem::Axis::Horizontal)
    .cross_axis_alignment(CrossAxisAlignment::Center)
    .main_axis_alignment(MainAxisAlignment::Center)
}

fn main() -> Result<(), EventLoopError> {
    let app = Xilem::new(AppState::default(), app_logic);
    app.run_windowed(EventLoop::with_user_event(), "Centered Flex".into())?;
    Ok(())
}
