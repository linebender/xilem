// Copyright 2023 the Xilem Authors
// SPDX-License-Identifier: Apache-2.0

use xilem_web2::{
    document_body,
    elements::html as el,
    interfaces::{Element, HtmlButtonElement, HtmlDivElement},
    App,
};

#[derive(Default)]
struct AppState {
    clicks: i32,
    class: Option<&'static str>,
    text: String,
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
    fn change_class(&mut self) {
        if self.class == Some("gray") {
            self.class = Some("green");
        } else {
            self.class = Some("gray");
        }
    }

    fn change_text(&mut self) {
        if self.text == "test" {
            self.text = "test2".into();
        } else {
            self.text = "test".into();
        }
    }
}

/// You can create functions that generate views.
fn btn(
    label: &'static str,
    click_fn: impl Fn(&mut AppState, web_sys::MouseEvent) + 'static,
) -> impl HtmlButtonElement<AppState> {
    el::button(label).on_click(click_fn)
}

fn app_logic(state: &mut AppState) -> impl HtmlDivElement<AppState> {
    el::div((
        el::span(format!("clicked {} times", state.clicks)).class(state.class),
        el::br(()),
        btn("+1 click", |state, _| state.increment()),
        btn("-1 click", |state, _| state.decrement()),
        btn("reset clicks", |state, _| state.reset()),
        btn("a different class", |state, _| state.change_class()),
        btn("change text", |state, _| state.change_text()),
        el::br(()),
        state.text.clone(),
    ))
}

pub fn main() {
    console_error_panic_hook::set_once();
    let app = App::new(AppState::default(), app_logic);
    app.run(&document_body());
}
