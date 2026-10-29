// Copyright 2026 the Xilem Authors
// SPDX-License-Identifier: Apache-2.0

//! This example shows how to create generic components.

use time::{Date, format_description::FormatItem, macros::format_description};
use xilem_web::{
    App,
    core::{MessageResult, map_action, map_message_result, map_state},
    document_body,
    elements::html,
    interfaces::Element,
};

// These modules could also be in an external crate.
mod card;
mod date_picker;

const DATE_FORMAT: &[FormatItem<'static>] = format_description!("[year]-[day]-[month]");

#[derive(Default)]
struct AppState {
    clicks: i32,
    card_collapsed: bool,
    date: Option<Date>,
    date_picker: date_picker::State,
}

impl AppState {
    fn click(&mut self) {
        self.clicks += 1;
    }

    fn toggle_card(&mut self) {
        self.card_collapsed = !self.card_collapsed;
    }

    fn show_date_picker(&mut self) {
        self.date_picker.popup_open = true;
    }

    fn close_date_picker(&mut self) {
        self.date_picker.popup_open = false;
    }
}

fn app_logic(state: &mut AppState) -> impl Element<AppState> + use<> {
    let counter = html::button("click me!").on_click(|state: &mut AppState, _| state.click());

    let card_content = html::div(("Some content ... ", state.clicks, counter));

    let card = card::view("Card Example", state.card_collapsed, card_content);
    let card = map_action(card, map_card_action);

    let date_input = html::input(())
        .attr(
            "value",
            state
                .date
                .map(|date| date.format(DATE_FORMAT).unwrap())
                .unwrap_or_default(),
        )
        .class("date")
        .on_focus(|state: &mut AppState, _| state.show_date_picker());

    let date_picker = date_picker::view(&state.date_picker);
    let date_picker = map_state(date_picker, |state: &mut AppState| &mut state.date_picker);
    let date_picker = map_message_result(date_picker, handle_date_picker_message_result);

    html::div((card, html::div((date_input, date_picker))))
}

fn map_card_action(state: &mut AppState, action: card::Action<()>) {
    match action {
        card::Action::Toggle => state.toggle_card(),
        card::Action::Child(()) => {}
    }
}

fn handle_date_picker_message_result(
    state: &mut AppState,
    message_result: MessageResult<date_picker::Action>,
) -> MessageResult<()> {
    let MessageResult::Action(action) = message_result else {
        return message_result.map(|_| ());
    };
    match action {
        date_picker::Action::DateChanged(date) => {
            state.date = date;
            state.close_date_picker();
        }
        date_picker::Action::Cancelled => {
            state.close_date_picker();
        }
    }
    MessageResult::Action(())
}

fn main() {
    console_error_panic_hook::set_once();
    App::new(document_body(), AppState::default(), app_logic).run();
}
