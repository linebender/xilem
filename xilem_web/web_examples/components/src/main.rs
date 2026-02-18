// Copyright 2026 the Xilem Authors
// SPDX-License-Identifier: Apache-2.0

//! Shows creating components.

use xilem_web::core::map_action;
use xilem_web::elements::html;
use xilem_web::interfaces::Element;
use xilem_web::{Action, App, DomFragment, document_body};

#[derive(Default)]
struct AppState {
    clicks: i32,
    card_collapsed: bool,
}

enum CardAction<M> {
    Toggle,
    Child(M),
}

impl<M> Action for CardAction<M> {}

fn card<State, Child, ChildAction>(
    title: &'static str,
    collapsed: bool,
    content: Child,
) -> impl Element<State, CardAction<ChildAction>>
where
    Child: DomFragment<State, ChildAction>,
    State: 'static,
    ChildAction: 'static,
{
    let content = map_action(
        html::div(content)
            .class("content")
            .class(collapsed.then_some("hidden")),
        |_, msg| CardAction::Child(msg),
    );

    html::div((
        html::h3(title)
            .class("title")
            .on_click(|_, _| CardAction::Toggle),
        content,
    ))
    .class("card")
}

fn app_logic(state: &mut AppState) -> impl Element<AppState> + use<> {
    let card = map_action(
        card(
            "Card Example",
            state.card_collapsed,
            html::div((
                "Some content ...",
                state.clicks,
                html::button("click").on_click(|s: &mut AppState, _| {
                    s.clicks += 1;
                }),
            )),
        ),
        |state: &mut AppState, msg: CardAction<()>| match msg {
            CardAction::Toggle => state.card_collapsed = !state.card_collapsed,
            CardAction::Child(_) => {}
        },
    );

    html::div(card)
}

fn main() {
    console_error_panic_hook::set_once();
    App::new(document_body(), AppState::default(), app_logic).run();
}
