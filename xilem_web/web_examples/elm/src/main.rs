// Copyright 2024 the Xilem Authors
// SPDX-License-Identifier: Apache-2.0

//! Xilem supports several patterns for creating modular components.
//! You can also emulate the elm architecture for a subset of your app.
//! Though usually it's more idiomatic to modularize state with
//! [`map_state`](xilem_web::core::map_state) or [`lens`](xilem_web::core::lens).

use xilem_web::core::map_action;
use xilem_web::elements::html as el;
use xilem_web::interfaces::{Element, HtmlDivElement};
use xilem_web::{Action, App, document_body};

#[derive(Debug, Default)]
struct Model {
    count: i32,
}

#[derive(Debug)]
enum Message {
    Increment,
    Decrement,
}

impl Action for Message {}

fn update(model: &mut Model, message: Message) {
    log::debug!("Update model {model:?} by {message:?}");
    match message {
        Message::Increment => model.count += 1,
        Message::Decrement => model.count -= 1,
    }
    log::debug!("Model updated: {model:?}");
}

fn app_logic(model: &mut Model) -> impl HtmlDivElement<Model> + use<> {
    log::debug!("Render view");
    el::div((map_action(counter_view(model.count), update),))
}

fn counter_view<T: 'static>(count: i32) -> impl HtmlDivElement<T, Message> {
    el::div((
        el::label(format!("count: {count}")),
        el::button("+").on_click(|_, _| Message::Increment),
        el::button("-").on_click(|_, _| Message::Decrement),
    ))
}

fn main() {
    _ = console_log::init_with_level(log::Level::Debug);
    console_error_panic_hook::set_once();
    log::info!("Start web application");
    App::new(document_body(), Model::default(), app_logic).run();
}
