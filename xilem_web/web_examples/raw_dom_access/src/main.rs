// Copyright 2023 the Xilem Authors
// SPDX-License-Identifier: Apache-2.0

//! This example demonstrates a dirty hack that should be avoided,
//! and only used with extreme caution in cases where direct access
//! to the raw DOM nodes is necessary
//! (e.g. when using external JS libraries).
//!
//! Please also note that no rebuild is triggered
//! after a callback has been performed in
//! `after_build`, `after_rebuild` or `before_teardown`.

use xilem_web::core::one_of::Either;
use xilem_web::elements::html;
use xilem_web::interfaces::Element;
use xilem_web::{App, DomView, document_body};

#[derive(Default)]
struct AppState {
    focus: bool,
    show_input: bool,
}

fn app_logic(app_state: &mut AppState) -> impl Element<AppState> + use<> {
    html::div(if app_state.show_input {
        Either::A(html::div((
            html::button("remove input").on_click(|app_state: &mut AppState, _| {
                app_state.show_input = false;
            }),
            html::input(())
                .after_build(|_, _| {
                    log::debug!("element was build");
                })
                .after_rebuild(move |app_state: &mut AppState, el| {
                    log::debug!("element was re-build");
                    if app_state.focus {
                        let _ = el.focus();
                        // Reset `focus` to avoid calling `el.focus` on every rebuild.
                        app_state.focus = false; // NOTE: this does NOT trigger a rebuild.
                    }
                })
                .before_teardown(|_| {
                    log::debug!("element will be removed");
                }),
            html::button("Focus the input").on_click(|app_state: &mut AppState, _| {
                app_state.focus = true;
            }),
        )))
    } else {
        Either::B(
            html::button("show input").on_click(|app_state: &mut AppState, _| {
                app_state.show_input = true;
            }),
        )
    })
}

pub fn main() {
    _ = console_log::init_with_level(log::Level::Debug);
    console_error_panic_hook::set_once();
    App::new(document_body(), AppState::default(), app_logic).run();
}
