// Copyright 2024 the Xilem Authors
// SPDX-License-Identifier: Apache-2.0

//! This example shows how (external) tasks can send messages
//! to be able to change the app state.

use futures::{select, FutureExt};
use gloo_timers::future::TimeoutFuture;
use xilem_web::{
    concurrent::{async_repeat, AsyncRepeatProxy, ShutdownSignal},
    core::fork,
    core::one_of::Either,
    document_body,
    elements::html,
    interfaces::Element,
    App,
};

#[derive(Default)]
struct AppState {
    ping_count: usize,
    run: bool,
}

#[derive(Debug)]
enum Message {
    Ping,
}

async fn create_ping_task(proxy: AsyncRepeatProxy, shutdown_signal: ShutdownSignal) {
    log::debug!("Start ping task");
    let mut abort = shutdown_signal.into_future().fuse();

    #[allow(clippy::infinite_loop)]
    loop {
        // NOTE:
        // This is just to simulate some async behavior.
        // If you just need an interval, you should use
        // [`interval`](xilem_web::concurrent::interval) instead.
        let mut timeout = TimeoutFuture::new(1_000).fuse();

        select! {
            _  = timeout => {
                proxy.send_message(Message::Ping);
                continue;
            }
            _ = abort => {
                // The view no longer exists so
                // we can do e.g. a graceful shutdown here.
                break;
            }
        }
    }
    log::debug!("Stop ping task");
}

fn app_logic(state: &mut AppState) -> impl Element<AppState> {
    let task = async_repeat(
        create_ping_task,
        |state: &mut AppState, message: Message| match message {
            Message::Ping => {
                state.ping_count += 1;
            }
        },
    );

    html::div((
        format!("Ping count: {}", state.ping_count),
        if state.run {
            Either::A(fork(
                html::button("stop").on_click(|state: &mut AppState, _| {
                    state.run = false;
                }),
                task,
            ))
        } else {
            Either::B(html::button("start").on_click(|state: &mut AppState, _| {
                state.run = true;
            }))
        },
    ))
}

pub fn main() {
    _ = console_log::init_with_level(log::Level::Debug);
    console_error_panic_hook::set_once();
    log::info!("Start web application");
    App::new(document_body(), AppState::default(), app_logic).run();
}
