// Copyright 2024 the Xilem Authors
// SPDX-License-Identifier: Apache-2.0

use wasm_bindgen::{JsCast, UnwrapThrowExt};
use xilem_web::{
    core::{fork, one_of::Either},
    document_body,
    elements::html::*,
    interfaces::{Element, HtmlDivElement, HtmlImageElement, HtmlLabelElement},
    memoized_await, App,
};

use gloo_net::http::Request;
use serde::{Deserialize, Serialize};

const TOO_MANY_CATS: usize = 8;

#[derive(Clone, Debug, PartialEq, Eq, Serialize, Deserialize)]
pub struct Cat {
    pub url: String,
}

async fn fetch_cats(count: usize) -> Result<Vec<Cat>, gloo_net::Error> {
    log::debug!("Fetch {count} cats");
    if count == 0 {
        return Ok(Vec::new());
    }
    let url = format!("https://api.thecatapi.com/v1/images/search?limit={count}");
    Ok(Request::get(&url)
        .send()
        .await?
        .json::<Vec<Cat>>()
        .await?
        .into_iter()
        .take(count)
        .collect())
}

struct AppState {
    cats_to_fetch: usize,
    cats_are_being_fetched: bool,
    cats: Vec<Cat>,
    debounce_in_ms: usize,
    reset_debounce_on_update: bool,
    error: Option<String>,
}

impl Default for AppState {
    fn default() -> Self {
        Self {
            cats_to_fetch: 0,
            cats: Vec::new(),
            debounce_in_ms: 1000,
            cats_are_being_fetched: false,
            reset_debounce_on_update: true,
            error: None,
        }
    }
}

pub fn input_target<T>(event: &T) -> web_sys::HtmlInputElement
where
    T: JsCast,
{
    event
        .unchecked_ref::<web_sys::Event>()
        .target()
        .unwrap_throw()
        .unchecked_into::<web_sys::HtmlInputElement>()
}

fn app_logic(state: &mut AppState) -> impl HtmlDivElement<AppState> {
    let cat_images = state
        .cats
        .iter()
        .map(|cat| img(()).src(cat.url.clone()))
        .collect::<Vec<_>>();
    div((
        fieldset((
            legend("Cat fetch controls"),
            table((
                tr((
                    td(
                        label("Reset fetch debounce timeout when updating the cat count:")
                            .for_("reset-debounce-update"),
                    ),
                    td(input(())
                        .id("reset-debounce-update")
                        .attr("type", "checkbox")
                        .attr("checked", state.reset_debounce_on_update)
                        .on_input(|state: &mut AppState, event: web_sys::Event| {
                            state.reset_debounce_on_update = input_target(&event).checked();
                        })),
                )),
                tr((
                    td(label("Debounce timeout in ms:").for_("debounce-timeout-duration")),
                    td(input(())
                        .id("debounce-timeout-duration")
                        .attr("type", "number")
                        .attr("min", 0)
                        .attr("value", state.debounce_in_ms)
                        .on_input(|state: &mut AppState, ev: web_sys::Event| {
                            state.debounce_in_ms = input_target(&ev).value().parse().unwrap_or(0);
                        })),
                )),
                tr((
                    td(label("How many cats would you like?").for_("cat-count")),
                    td(input(())
                        .id("cat-count")
                        .attr("type", "number")
                        .attr("min", 0)
                        .attr("value", state.cats_to_fetch)
                        .on_input(|state: &mut AppState, ev: web_sys::Event| {
                            if !state.cats_are_being_fetched {
                                state.cats.clear();
                            }
                            state.cats_are_being_fetched = true;
                            state.cats_to_fetch = input_target(&ev).value().parse().unwrap_or(0);
                        })),
                )),
            )),
        ))
        .class("cat-fetch-controls"),
        state
            .error
            .as_ref()
            .map(|err| div((h2("Error"), p(err.to_string()))).class("error")),
        fork(
            div((
                if state.cats_to_fetch != 0 && state.cats_to_fetch == cat_images.len() {
                    Either::A(p("Here are your cats:").class("blink"))
                } else if state.cats_to_fetch >= TOO_MANY_CATS {
                    Either::B(p("Woah there, that's too many cats"))
                } else if state.debounce_in_ms > 0
                    && state.cats_to_fetch > 0
                    && state.reset_debounce_on_update
                {
                    Either::B(p("Debounced fetch of cats..."))
                } else if state.debounce_in_ms > 0 && state.cats_to_fetch > 0 {
                    Either::B(p("Throttled fetch of cats..."))
                } else if state.cats_to_fetch > 0 && state.cats_are_being_fetched {
                    Either::B(p("Fetching cats..."))
                } else {
                    Either::B(p("You need to fetch cats"))
                },
                cat_images,
            )),
            // Here's the actual fetching logic:
            (state.cats_to_fetch < TOO_MANY_CATS).then_some(
                memoized_await(
                    state.cats_to_fetch,
                    |count| fetch_cats(*count),
                    |state: &mut AppState, cats_result| match cats_result {
                        Ok(cats) => {
                            log::info!("Received {} cats", cats.len());
                            state.cats = cats;
                            state.cats_are_being_fetched = false;
                            state.error = None;
                        }
                        Err(err) => {
                            log::warn!("Unable to fetch cats: {err:#}");
                            state.cats_are_being_fetched = false;
                            state.error = Some(err.to_string());
                        }
                    },
                )
                .debounce(state.debounce_in_ms)
                .reset_debounce_on_update(state.reset_debounce_on_update),
            ),
        ),
    ))
}

pub fn main() {
    _ = console_log::init_with_level(log::Level::Debug);
    console_error_panic_hook::set_once();

    log::info!("Start application");

    App::new(document_body(), AppState::default(), app_logic).run();
}
