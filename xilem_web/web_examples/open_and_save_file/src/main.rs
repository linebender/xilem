// Copyright 2024 the Xilem Authors
// SPDX-License-Identifier: Apache-2.0

//! This example demonstrates how to open or save a text file
//! within a client side rendered web application without a server.

use gloo_file::{Blob, File, FileReadError, ObjectUrl};
use web_sys::wasm_bindgen::JsCast;
use xilem_web::{
    App, DomView,
    concurrent::memoized_await,
    core::{Edit, fork},
    document_body,
    elements::html,
    interfaces::{Element, HtmlInputElement},
    modifiers::style,
    textarea_event_target_value,
};

struct AppState {
    text: String,
    file_to_open: Option<File>,
    start_opening: bool,
    start_saving: bool,
}

impl Default for AppState {
    fn default() -> Self {
        AppState {
            text: "Hello from Xilem Web :)".to_string(),
            file_to_open: None,
            start_opening: false,
            start_saving: false,
        }
    }
}

fn app_logic(state: &mut AppState) -> impl Element<Edit<AppState>> + use<> {
    let open_action = state
        .start_opening
        .then(|| {
            state.file_to_open.take().map(|file| {
                state.file_to_open = None;
                state.start_opening = false;
                memoized_await(
                    file,
                    |file| gloo_file::futures::read_as_text(file),
                    handle_open_result,
                )
            })
        })
        .flatten();

    html::div((
        html::h1("Open and save file example"),
        html::textarea(state.text.clone()).on_input(|state: &mut AppState, ev| {
            let Some(text) = textarea_event_target_value(&ev) else {
                return;
            };
            state.text = text;
        }),
        html::h2("Save"),
        html::button("save text").on_click(|state: &mut AppState, _| state.start_saving = true),
        hidden_save_link(),
        html::h2("Open"),
        html::div((
            open_file_input(),
            html::button("x").on_click(|state: &mut AppState, _| state.file_to_open = None),
        )),
        fork(
            html::button("open").on_click(|state: &mut AppState, _| state.start_opening = true),
            open_action,
        ),
    ))
}

fn handle_open_result(state: &mut AppState, result: Result<String, FileReadError>) {
    match result {
        Ok(txt) => {
            state.text = txt;
        }
        Err(err) => {
            log::error!("Unable to open file: {err}");
        }
    }
}

fn open_file_input() -> impl Element<Edit<AppState>> + use<> {
    html::input(())
        .type_("file")
        .attr("accept", "text/plain")
        .on_change(|state: &mut AppState, ev| {
            ev.prevent_default();
            let input = ev
                .target()
                .unwrap()
                .unchecked_into::<web_sys::HtmlInputElement>();
            let Some(files) = input.files() else {
                state.file_to_open = None;
                return;
            };
            state.file_to_open = files.get(0).map(File::from);
        })
        .after_rebuild(|state: &mut AppState, el| {
            if state.file_to_open.is_none() {
                el.set_value("");
            }
        })
}

fn hidden_save_link() -> impl Element<Edit<AppState>> + use<> {
    html::a("Save example text")
        .style(style("display", "none"))
        .attr("download", "example.txt")
        .after_rebuild(|state: &mut AppState, el| {
            if !state.start_saving {
                return;
            }
            state.start_saving = false;
            let blob = Blob::new(&*state.text);
            let url = ObjectUrl::from(blob);
            el.set_href(&url);
            el.click();
        })
}

pub fn main() {
    _ = console_log::init_with_level(log::Level::Debug);
    console_error_panic_hook::set_once();
    App::new(document_body(), AppState::default(), app_logic).run();
}
