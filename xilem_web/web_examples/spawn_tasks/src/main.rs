// Copyright 2024 the Xilem Authors
// SPDX-License-Identifier: Apache-2.0

//! This example shows how (external) tasks can send messages
//! to be able to change the app state.

#![expect(clippy::shadow_unrelated, reason = "Idiomatic for Xilem users")]

use futures::{FutureExt, select};
use gloo_timers::future::TimeoutFuture;
use xilem_web::concurrent::{ShutdownSignal, TaskProxy, task};
use xilem_web::core::fork;
use xilem_web::core::one_of::Either;
use xilem_web::elements::html;
use xilem_web::interfaces::Element;
use xilem_web::{App, document_body};

#[derive(Default)]
struct AppState {
    ping_count: usize,
    run: bool,
}

#[derive(Debug)]
enum Message {
    Ping,
}

/// NOTE:
/// This is just to simulate some async behavior.
/// If you just need an interval, you should use
/// [`interval`](xilem_web::concurrent::interval) instead.
async fn create_ping_task(proxy: TaskProxy, shutdown_signal: ShutdownSignal) {
    log::debug!("Start ping task");
    let mut abort = shutdown_signal.into_future().fuse();

    loop {
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

fn app_logic(state: &mut AppState) -> impl Element<AppState> + use<> {
    let task = task(
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
fn main() {
    _ = console_log::init_with_level(log::Level::Debug);
    console_error_panic_hook::set_once();
    log::info!("Start web application");
    App::new(document_body(), AppState::default(), app_logic).run();
}
