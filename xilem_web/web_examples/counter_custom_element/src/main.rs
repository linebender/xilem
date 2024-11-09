// Copyright 2023 the Xilem Authors
// SPDX-License-Identifier: Apache-2.0

//! Shows creating a element by raw tag name. This can be useful for web components

#![expect(clippy::shadow_unrelated, reason = "Idiomatic for Xilem users")]

use xilem_web::{
    document_body,
    elements::custom_element,
    interfaces::{Element, HtmlElement},
    App, DomView,
};

#[derive(Default)]
struct AppState {
    clicks: i32,
}

impl AppState {
    fn increment(&mut self) {
        self.clicks += 1;
    }
    fn decrement(&mut self) {
        self.clicks -= 1;
    }
    fn reset(&mut self) {
        self.clicks = 0;
    }
}

fn btn(
    label: &'static str,
    click_fn: impl Fn(&mut AppState, web_sys::Event) + 'static,
) -> impl HtmlElement<AppState> {
    custom_element("button", label).on("click", move |state: &mut AppState, evt| {
        click_fn(state, evt);
    })
}

fn app_logic(state: &mut AppState) -> impl DomView<AppState> {
    custom_element(
        "div",
        (
            custom_element("span", format!("clicked {} times", state.clicks)),
            btn("+1 click", |state, _| AppState::increment(state)),
            btn("-1 click", |state, _| AppState::decrement(state)),
            btn("reset clicks", |state, _| AppState::reset(state)),
        ),
    )
}

fn main() {
    console_error_panic_hook::set_once();
    App::new(document_body(), AppState::default(), app_logic).run();
}
