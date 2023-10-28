// Copyright 2023 the Xilem Authors
// SPDX-License-Identifier: Apache-2.0

use xilem_web::{document_body, elements::html, interfaces::Element, App};

type AppState = bool;

fn app_logic(_state: &mut AppState) -> impl Element<AppState> {
    html::div((
        html::input(()), // FIXME
        // .after_update(|focus_input, el| {
        //     if *focus_input {
        //         let _ = el.node.focus();
        //         *focus_input = false;
        //     }
        // })
        html::button("Focus the input").on_click(|focus_input: &mut bool, _| {
            *focus_input = true;
        }),
    ))
}

pub fn main() {
    console_error_panic_hook::set_once();
    App::new(document_body(), AppState::default(), app_logic).run();
}
