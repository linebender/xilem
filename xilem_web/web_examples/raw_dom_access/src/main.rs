// Copyright 2023 the Xilem Authors
// SPDX-License-Identifier: Apache-2.0

use xilem_web::{DomView, core::one_of::Either, document_body, elements::html, interfaces::Element, App};

#[derive(Default)]
struct AppState {
    focus: bool,
    show_input: bool,
}

fn app_logic(app_state: &mut AppState) -> impl Element<AppState> {
    html::div(if app_state.show_input {
        let focus = app_state.focus;
        Either::A(html::div((
            html::button("remove input").on_click(|app_state: &mut AppState, _| {
                app_state.show_input = false;
            }),
            html::input(())
                .after_build(|_| {
                    log::debug!("element was build");
                })
                .after_rebuild(move |el| {
                    log::debug!("element was re-build");
                    if focus {
                        el.focus().unwrap();
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
