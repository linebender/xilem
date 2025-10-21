// Copyright 2025 the Xilem Authors
// SPDX-License-Identifier: Apache-2.0

//! An example of an app with multiple windows.

// On Windows platform, don't show a console when opening the app.
#![windows_subsystem = "windows"]

use std::collections::{BTreeMap, HashMap};

use winit::error::EventLoopError;
use xilem::view::{text_button, flex_col, label, text_input};
use xilem::{AppState, EventLoop, EventLoopBuilder, WindowId, WindowView, Xilem, window};

struct State {
    new_counter_name: String,
    counters: HashMap<WindowId, Counter>,
    running: bool,
    main_window_id: WindowId,
}

struct Counter {
    name: String,
    value: isize,
}

impl AppState for State {
    fn keep_running(&self) -> bool {
        self.running
    }
}

fn app_logic(state: &mut State) -> impl Iterator<Item = WindowView<State>> + use<> {
    std::iter::once(
        window(
            state.main_window_id,
            "Multiple windows",
            flex_col((
                label(format!(
                    "{:#?}",
                    state
                        .counters
                        .values()
                        .map(|counter| (&counter.name, counter.value))
                        .collect::<BTreeMap<_, _>>()
                )),
                text_input(
                    state.new_counter_name.clone(),
                    |state: &mut State, new_name| {
                        state.new_counter_name = new_name;
                    },
                ),
                text_button("Add".to_string(), |state: &mut State| {
                    if state
                        .counters
                        .values()
                        .any(|counter| counter.name == state.new_counter_name)
                    {
                        // TODO: show error if name already exists
                        return;
                    }
                    let name = std::mem::take(&mut state.new_counter_name);
                    state
                        .counters
                        .insert(WindowId::next(), Counter { name, value: 0 });
                }),
            )),
        )
        .with_options(|o| o.on_close(|state: &mut State| state.running = false)),
    )
    .chain(
        state
            .counters
            .iter()
            .map(|(window_id, Counter { name, value })| {
                let window_id = *window_id;
                window(
                    window_id,
                    name,
                    flex_col((
                        label(format!("count: {value}")),
                        text_button("+".to_string(), move |state: &mut State| {
                            state.counters.get_mut(&window_id).unwrap().value += 1;
                        }),
                        text_button("-".to_string(), move |state: &mut State| {
                            state.counters.get_mut(&window_id).unwrap().value -= 1;
                        }),
                    )),
                )
                .with_options(|o| {
                    o.on_close(move |state: &mut State| {
                        state.counters.remove(&window_id);
                    })
                })
            }),
    )
    .collect::<Vec<_>>()
    .into_iter()
}

fn run(event_loop: EventLoopBuilder) -> Result<(), EventLoopError> {
    let data = State {
        new_counter_name: String::new(),
        counters: HashMap::new(),
        running: true,
        main_window_id: WindowId::next(),
    };
    let app = Xilem::new(data, app_logic);
    app.run_in(event_loop)
}

fn main() -> Result<(), EventLoopError> {
    run(EventLoop::with_user_event())
}
