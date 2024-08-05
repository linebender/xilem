// Copyright 2024 the Xilem Authors
// SPDX-License-Identifier: Apache-2.0

use gloo_timers::future::TimeoutFuture;
use xilem_web::{
    concurrent::async_repeat_raw, core::fork, document_body, elements::html, interfaces::Element,
    App,
};

#[derive(Default)]
struct AppState {
    ping_count: usize,
}

#[derive(Debug)]
enum Message {
    Ping,
}

fn app_logic(state: &mut AppState) -> impl Element<AppState> {
    let task = async_repeat_raw(
        |thunk| async move {
            loop {
                TimeoutFuture::new(1_000).await;
                thunk.push_message(Message::Ping);
            }
        },
        |state: &mut AppState, message: Message| match message {
            Message::Ping => {
                state.ping_count += 1;
            }
        },
    );

    fork(html::div(format!("Ping count: {}", state.ping_count)), task)
}

pub fn main() {
    _ = console_log::init_with_level(log::Level::Debug);
    console_error_panic_hook::set_once();
    log::info!("Start web application");
    App::new(document_body(), AppState::default(), app_logic).run();
}
