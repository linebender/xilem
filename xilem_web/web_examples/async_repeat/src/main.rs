// Copyright 2024 the Xilem Authors
// SPDX-License-Identifier: Apache-2.0

use futures::{select, FutureExt};
use gloo_timers::future::TimeoutFuture;
use xilem_web::{
    concurrent::async_repeat, core::fork, core::one_of::Either, document_body, elements::html,
    interfaces::Element, App,
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

fn app_logic(state: &mut AppState) -> impl Element<AppState> {
    let task = async_repeat(
        |proxy, shutdown_signal| async move {
            log::debug!("Start ping task");

            let mut abort = shutdown_signal.into_future().fuse();

            #[allow(clippy::infinite_loop)]
            loop {
                let mut timeout = TimeoutFuture::new(1_000).fuse();
                select! {
                   _  = timeout => {
                      proxy.send_message(Message::Ping);
                      continue;
                  }
                   _ = abort => {
                        log::debug!("Stop ping task");
                        break;
                   }
                }
            }
        },
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
