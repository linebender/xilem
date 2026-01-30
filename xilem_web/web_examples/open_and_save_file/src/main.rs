// Copyright 2024 the Xilem Authors
// SPDX-License-Identifier: Apache-2.0

//! This example demonstrates how to open or save a text file
//! within a client side rendered web application without a server.

use std::{cell::RefCell, rc::Rc};

use gloo_file::{Blob, File, FileReadError, ObjectUrl};
use web_sys::wasm_bindgen::JsCast;
use xilem_web::{
    App, DomView,
    concurrent::memoized_await,
    core::{Edit, fork},
    document_body,
    elements::html,
    interfaces::Element,
    modifiers::style,
};

struct AppState {
    text: String,
    file_to_open: Option<File>,
    start_opening: bool,
    raw_file_input_el: Rc<RefCell<Option<web_sys::HtmlInputElement>>>,
    raw_save_link: Rc<RefCell<Option<web_sys::HtmlAnchorElement>>>,
}

impl Default for AppState {
    fn default() -> Self {
        AppState {
            text: "Hello from Xilem Web :)".to_string(),
            file_to_open: None,
            start_opening: false,
            raw_file_input_el: Rc::new(RefCell::new(None)),
            raw_save_link: Rc::new(RefCell::new(None)),
        }
    }
}

fn app_logic(app_state: &mut AppState) -> impl Element<Edit<AppState>> + use<> {
    let open_action = app_state
        .start_opening
        .then(|| {
            app_state.file_to_open.take().map(|file| {
                reset_file_input(app_state);
                app_state.start_opening = false;
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
        html::textarea(app_state.text.clone()),
        html::h2("Save"),
        html::button("save text").on_click(|state: &mut AppState, _| {
            let el_ref = state.raw_save_link.borrow_mut();
            let blob = Blob::new(&*state.text);
            let url = ObjectUrl::from(blob);
            let el = el_ref.as_ref().unwrap();
            el.set_href(&url);
            el.click();
        }),
        hidden_save_link(app_state),
        html::h2("Open"),
        html::div((
            open_file_input(app_state),
            html::button("x").on_click(|state: &mut AppState, _| {
                reset_file_input(state);
            }),
        )),
        fork(
            html::button("open").on_click(|state: &mut AppState, _| {
                state.start_opening = true;
            }),
            open_action,
        ),
    ))
}

fn reset_file_input(state: &mut AppState) {
    state.file_to_open = None;
    if let Some(el) = &*state.raw_file_input_el.borrow_mut() {
        el.set_value("");
    }
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

fn open_file_input(app_state: &mut AppState) -> impl Element<Edit<AppState>> + use<> {
    html::input(())
        .attr("type", "file")
        .attr("accept", "text/plain")
        .after_build({
            let el_ref = Rc::clone(&app_state.raw_file_input_el);
            move |el| {
                *el_ref.borrow_mut() = Some(el.clone());
            }
        })
        .before_teardown({
            let el_ref = Rc::clone(&app_state.raw_file_input_el);
            move |_| {
                *el_ref.borrow_mut() = None;
            }
        })
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
}

fn hidden_save_link(state: &mut AppState) -> impl Element<Edit<AppState>> + use<> {
    html::a("Save example text")
        .style(style("display", "none"))
        .attr("save", "example.txt")
        .after_build({
            let el_ref = Rc::clone(&state.raw_save_link);
            move |el| {
                *el_ref.borrow_mut() =
                    Some(el.dyn_ref::<web_sys::HtmlAnchorElement>().unwrap().clone());
            }
        })
        .before_teardown({
            let el_ref = Rc::clone(&state.raw_save_link);
            move |_| {
                *el_ref.borrow_mut() = None;
            }
        })
}

pub fn main() {
    _ = console_log::init_with_level(log::Level::Debug);
    console_error_panic_hook::set_once();
    App::new(document_body(), AppState::default(), app_logic).run();
}
