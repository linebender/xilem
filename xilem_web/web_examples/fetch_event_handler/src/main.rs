// Copyright 2024 the Xilem Authors
// SPDX-License-Identifier: Apache-2.0

use wasm_bindgen::{JsCast, UnwrapThrowExt as _};
use xilem_web::{
    document_body,
    elements::html::*,
    event_handler::defer,
    interfaces::{Element, HtmlDivElement, HtmlImageElement},
    App,
};

use gloo_net::http::Request;
use serde::{Deserialize, Serialize};
use thiserror::Error;

#[derive(Clone, Debug, PartialEq, Eq, Serialize, Deserialize)]
pub struct Cat {
    pub url: String,
}

#[derive(Error, Clone, Debug)]
pub enum CatError {
    #[error("Please request more than zero cats.")]
    NonZeroCats,
}

pub type CatCount = usize;

pub async fn fetch_cats(count: CatCount) -> anyhow::Result<Vec<Cat>> {
    log::debug!("Fetch {count} cats");
    if count < 1 {
        return Err(CatError::NonZeroCats.into());
    }
    let url = format!("https://api.thecatapi.com/v1/images/search?limit={count}",);
    Ok(Request::get(&url)
        .send()
        .await?
        .json::<Vec<Cat>>()
        .await?
        .into_iter()
        .take(count)
        .collect())
}

#[derive(Default)]
struct AppState {
    cat_count: usize,
    cats: Vec<Cat>,
    error: Option<String>,
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
                .on_input(defer(
                    |state: &mut AppState, ev: web_sys::Event| {
                        let count = event_target_value(&ev).parse::<CatCount>().unwrap_or(0);
                        state.cat_count = count;
                        state.cats.clear();
                        fetch_cats(count)
                    },
                    |state: &mut AppState, fetch_result| match fetch_result {
                        Ok(cats) => {
                            log::info!("Received {} cats", cats.len());
                            state.cats = cats;
                            state.error = None;
                        }
                        Err(err) => {
                            log::warn!("Unable to fetch cats: {err:#}");
                            state.error = Some(err.to_string());
                        }
                    },
                )),
        )),
        state
            .error
            .as_ref()
            .map(|err| div((h2("Error"), p(err.to_string()))).class("error")),
        cats,
    ))
}

pub fn main() {
    _ = console_log::init_with_level(log::Level::Debug);
    console_error_panic_hook::set_once();

    log::info!("Start application");

    App::new(document_body(), AppState::default(), app_logic).run();
}
