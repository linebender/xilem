// Copyright 2024 the Xilem Authors
// SPDX-License-Identifier: Apache-2.0

//! This example demonstrates the use of [`memoized_stream`].

use std::pin::Pin;

use futures::{stream, Stream};
use xilem_web::{
    concurrent::memoized_stream, core::fork, document_body, elements::html,
    input_event_target_value, interfaces::Element, App,
};

mod api;
use self::api::{AbortHandle, MockConnection, StreamMessage};

#[derive(Default)]
struct AppState {
    search_term: String,
    db: MockConnection,
    current_search: Option<AbortHandle>,
}

const DEBOUNCE_MILLIS: usize = 500;

fn app_logic(state: &mut AppState) -> impl Element<AppState> {
    log::debug!("Run app logic");

    let search_stream = memoized_stream(
        state.search_term.clone(),
        create_search_stream,
        handle_stream_message,
    )
    .debounce_ms(DEBOUNCE_MILLIS)
    .reset_debounce_on_update(true);

    html::div(fork(
        html::div((
            html::input(())
                .on_keyup(on_search_input_keyup)
                .attr("value", state.search_term.clone())
                .attr("placeholder", "Type to search"),
            html::p((
                "Search is running: ",
                if state.current_search.is_some() {
                    "yes"
                } else {
                    "no"
                },
            )),
        )),
        search_stream,
    ))
}

fn on_search_input_keyup(state: &mut AppState, ev: web_sys::KeyboardEvent) {
    if ev.key() == "Escape" {
        state.search_term.clear();
        return;
    }
    if let Some(abort_handle) = state.current_search.take() {
        abort_handle.abort();
    };
    state.search_term = input_event_target_value(&ev).expect("input value");
}

fn create_search_stream(
    state: &mut AppState,
    term: &String,
) -> Pin<Box<dyn Stream<Item = StreamMessage>>> {
    if term.is_empty() {
        Box::pin(stream::empty())
    } else {
        Box::pin(state.db.search(term.to_owned()))
    }
}

fn handle_stream_message(state: &mut AppState, message: StreamMessage) {
    match message {
        StreamMessage::Started(abort_handle) => {
            log::debug!("Search stream started");
            debug_assert!(
                state.current_search.is_none(),
                "The previous search should already have been canceled"
            );
            state.current_search = Some(abort_handle);
        }
        StreamMessage::SearchResult(result) => {
            log::debug!("Stream result {result:?}");
        }
        StreamMessage::Aborted => {
            log::debug!("Stream aborted");
            state.current_search = None;
        }
        StreamMessage::TimedOut => {
            log::debug!("Stream timed out");
            state.current_search = None;
        }
        StreamMessage::Finished => {
            log::debug!("Stream finished");
            state.current_search = None;
        }
    }
}

fn main() {
    _ = console_log::init_with_level(log::Level::Debug);
    console_error_panic_hook::set_once();
    log::info!("Start application");
    App::new(document_body(), AppState::default(), app_logic).run();
}
