// Copyright 2024 the Xilem Authors
// SPDX-License-Identifier: Apache-2.0

//! This example is inspired by the "fecth" example from https://github.com/leptos-rs/leptos

use futures::{channel::mpsc, SinkExt, StreamExt};
use wasm_bindgen::{JsCast, UnwrapThrowExt};
use wasm_bindgen_futures::spawn_local;

use xilem_web::{
    document_body,
    elements::html::*,
    interfaces::{Element, HtmlDivElement, HtmlImageElement},
    App,
};

mod api;
use self::api::*;

struct AppState {
    cat_count: usize,
    cats: Vec<Cat>,
    error: Option<String>,
    msg_tx: mpsc::Sender<Message>,
}

impl AppState {
    fn new(msg_tx: mpsc::Sender<Message>) -> Self {
        Self {
            cat_count: 0,
            cats: Vec::new(),
            error: None,
            msg_tx,
        }
    }
}

fn app_logic(state: &mut AppState) -> impl HtmlDivElement<AppState> {
    let cats = state
        .cats
        .iter()
        .map(|cat| p(img(()).src(cat.url.clone())))
        .collect::<Vec<_>>();
    div((
        label((
            "How many cats would you like?",
            input(())
                .attr("type", "number")
                .attr("value", state.cat_count.to_string())
                .on_input(move |state: &mut AppState, ev| {
                    let count = event_target_value(&ev).parse::<CatCount>().unwrap_or(0);
                    state.cat_count = count;
                    let mut tx = state.msg_tx.clone();
                    spawn_local(async move {
                        let result = fetch_cats(count).await;
                        let msg = Message::FetchCatsResult(result);
                        drop(tx.send(msg).await);
                    });
                }),
        )),
        state
            .error
            .as_ref()
            .map(|err| div((h2("Error"), p(err.to_string()))).class("error")),
        cats,
    ))
}

pub fn event_target_value<T>(event: &T) -> String
where
    T: JsCast,
{
    event
        .unchecked_ref::<web_sys::Event>()
        .target()
        .unwrap_throw()
        .unchecked_into::<web_sys::HtmlInputElement>()
        .value()
}

enum Message {
    FetchCatsResult(anyhow::Result<Vec<Cat>>),
}

pub fn main() {
    _ = console_log::init_with_level(log::Level::Debug);
    console_error_panic_hook::set_once();

    log::info!("Start application");
    let (cmd_tx, mut cmd_rx) = mpsc::channel::<Message>(100);
    let app = App::new(document_body(), AppState::new(cmd_tx), app_logic);

    spawn_local({
        let app = app.clone();
        async move {
            log::debug!("Start message loop");
            while let Some(msg) = cmd_rx.next().await {
                match msg {
                    Message::FetchCatsResult(Ok(cats)) => {
                        log::info!("Received {} cats", cats.len());
                        app.modify_state(move |state| {
                            state.cats = cats;
                        });
                    }
                    Message::FetchCatsResult(Err(err)) => {
                        log::warn!("Unable to fetch cats: {err:#}");
                        app.modify_state(move |state| {
                            state.error = Some(err.to_string());
                        });
                    }
                }
            }
            log::debug!("Terminated message loop");
        }
    });

    app.run();
}
