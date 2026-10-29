// Copyright 2026 the Xilem Authors
// SPDX-License-Identifier: Apache-2.0

//! This example shows how to communicate with external systems
//! via messages that can change the app state.

use std::fmt;

use futures_channel::mpsc;
use futures_util::{
    FutureExt, StreamExt,
    future::{self, Either},
};
use gloo_timers::future::TimeoutFuture;
use wasm_bindgen_futures::spawn_local;
use xilem_web::{
    App,
    concurrent::{ShutdownSignal, TaskProxy, task_raw},
    core::{Edit, fork, one_of},
    document_body,
    elements::html,
    interfaces::Element,
};

const CHANNEL_SIZE: usize = 5;

enum ExternalMessage {
    HelloFromExtern,
}

enum XilemMessage {
    HelloFromXilem,
    StartReceiving {
        msg_tx: mpsc::Sender<ExternalMessage>,
    },
}

// We assume that the external message does not have `Debug` implemented,
// so we need a wrapper because `TaskProxy::send_message` requires
// the message to implement `Debug`.
struct MessageWrapper(ExternalMessage);

impl fmt::Debug for MessageWrapper {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        write!(f, "MessageWrapper(..)")
    }
}

struct AppState {
    msg_tx: mpsc::Sender<XilemMessage>,
    message_count: usize,
    run: bool,
}

impl AppState {
    fn new() -> (Self, mpsc::Receiver<XilemMessage>) {
        let (msg_tx, msg_rx) = mpsc::channel::<XilemMessage>(CHANNEL_SIZE);
        (
            Self {
                msg_tx,
                message_count: 0,
                run: false,
            },
            msg_rx,
        )
    }
}

async fn create_receiver_task(
    proxy: TaskProxy,
    shutdown_signal: ShutdownSignal,
    mut msg_tx: mpsc::Sender<XilemMessage>,
) {
    log::debug!("Xilem: start message receiver task");
    let mut abort = shutdown_signal.into_future().fuse();
    let (external_msg_tx, mut msg_rx) = mpsc::channel::<ExternalMessage>(CHANNEL_SIZE);
    let msg = XilemMessage::StartReceiving {
        msg_tx: external_msg_tx,
    };
    if msg_tx.try_send(msg).is_err() {
        log::info!("Xilem: No external receiver anymore; stop receiver task");
        return;
    };
    log::debug!("Xilem: start receiving messages");
    loop {
        match future::select(msg_rx.next(), &mut abort).await {
            Either::Left((Some(msg), _)) => {
                proxy.send_message(MessageWrapper(msg));
                continue;
            }
            Either::Left((None, _)) => {
                // There is no sender anymore.
                break;
            }
            Either::Right(_) => {
                // The view no longer exists so
                // we can do e.g. a graceful shutdown here.
                break;
            }
        }
    }
    log::debug!("Xilem: stop message receiver task");
}

fn handle_message(state: &mut AppState, MessageWrapper(msg): MessageWrapper) {
    match msg {
        ExternalMessage::HelloFromExtern => {
            log::info!("Xilem: Hello from extern");
            state.message_count += 1;
        }
    }
}

fn app_logic(state: &mut AppState) -> impl Element<Edit<AppState>> + use<> {
    let tx = state.msg_tx.clone();
    let task = task_raw::<_, _, _, Edit<AppState>, _, _>(
        move |proxy, shutdown_signal| create_receiver_task(proxy, shutdown_signal, tx.clone()),
        handle_message,
    );

    html::div((
        format!("Message count: {}", state.message_count),
        if state.run {
            one_of::Either::A(fork(
                html::button("stop receiving").on_click(|state: &mut AppState, _| {
                    state.run = false;
                }),
                task,
            ))
        } else {
            one_of::Either::B(html::button("start receiving").on_click(
                |state: &mut AppState, _| {
                    state.run = true;
                },
            ))
        },
        html::button("Hello to Extern").on_click(|state: &mut AppState, _| {
            if state.msg_tx.try_send(XilemMessage::HelloFromXilem).is_err() {
                log::warn!("Xilem: No external receiver");
            }
        }),
    ))
}

async fn external_message_sender_task(mut msg_tx: mpsc::Sender<ExternalMessage>) {
    log::debug!("Extern: start message sender task");
    loop {
        let timeout = TimeoutFuture::new(1_000).fuse();
        timeout.await;
        if msg_tx.try_send(ExternalMessage::HelloFromExtern).is_err() {
            // The receiver within xilem is no listening anymore.
            break;
        }
    }
    log::debug!("Extern: stop message sender task");
}

async fn external_message_receiver_task(mut msg_rx: mpsc::Receiver<XilemMessage>) {
    log::debug!("Extern: start message receiver task");

    let mut tx: Option<mpsc::Sender<ExternalMessage>> = None;

    while let Some(msg) = msg_rx.next().await {
        match msg {
            XilemMessage::HelloFromXilem => {
                log::info!("Extern: Hello from Xilem :)");
                let Some(msg_tx) = &mut tx else {
                    continue;
                };
                if msg_tx.try_send(ExternalMessage::HelloFromExtern).is_err() {
                    log::debug!("Extern: no receiver anymore");
                    tx = None;
                }
            }
            XilemMessage::StartReceiving { msg_tx } => {
                tx = Some(msg_tx.clone());
                spawn_local(external_message_sender_task(msg_tx));
            }
        }
    }
    log::debug!("Extern: stop message receiver task");
}

fn main() {
    _ = console_log::init_with_level(log::Level::Debug);
    console_error_panic_hook::set_once();
    log::info!("Start web application");

    let (initial_state, msg_rx) = AppState::new();
    App::new(document_body(), initial_state, app_logic).run();
    spawn_local(external_message_receiver_task(msg_rx));
}
