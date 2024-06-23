// Copyright 2024 the Xilem Authors
// SPDX-License-Identifier: Apache-2.0

use wasm_bindgen::{JsCast, UnwrapThrowExt};
use xilem_web::{
    core::{memoize, one_of::OneOf2},
    document_body,
    elements::html::{button, div, h1, h2, img, label, option, p, select},
    interfaces::{
        Element, HtmlButtonElement, HtmlElement, HtmlImageElement, HtmlLabelElement,
        HtmlOptionElement,
    },
    style as s, App,
};

const LOREM_IPSUM: &str = "Lorem ipsum dolor sit amet, consectetur adipiscing elit, sed do eiusmod tempor incididunt ut labore et dolore magna aliqua. Ut enim ad minim veniam, quis nostrud exercitation ullamco laboris nisi ut aliquip ex ea commodo consequat. Duis aute irure dolor in reprehenderit in voluptate velit esse cillum dolore eu fugiat nulla pariatur. Excepteur sint occaecat cupidatat non proident, sunt in culpa qui officia deserunt mollit anim id est laborum.";

struct BlogState {
    current_page: u32,
    posts_per_page: u32,
}

impl Default for BlogState {
    fn default() -> Self {
        BlogState {
            current_page: 0,
            posts_per_page: 5,
        }
    }
}

impl BlogState {
    fn next_page(&mut self) {
        self.current_page += 1;
    }
    fn previous_page(&mut self) {
        self.current_page = self.current_page.saturating_sub(1);
    }
}

fn controls(state: &mut BlogState) -> impl HtmlElement<BlogState> {
    // This whole view is only dependent on posts_per_page and whether we are on the first page.
    // To optimize this, this can and probably should be memoized based on these properties.
    // Although all of the following is memoized, the top-level element in this memoize (`div`) returned could be styled further externally as seen below
    memoize(
        (state.posts_per_page, state.current_page == 0),
        |(posts_per_page, on_first_page)| {
            let page_change_button = |label, callback: fn(&mut BlogState)| {
                button(label)
                    .on_click(move |state, _| callback(state))
                    .class("page-change-button")
            };

            let posts_per_page_select = div((
                label("Posts per page:").for_("posts-per-page-select"),
                select([3, 5, 10, 25].map(|count| {
                    option(count)
                        .selected(count == *posts_per_page)
                        .value(count)
                }))
                .id("posts-per-page-select")
                .on_change(|state: &mut BlogState, event| {
                    let count = event
                        // TODO: We probably want sugarized specialized event handlers in general, to avoid all this boilerplate
                        // (In this case maybe something like `on_value_change`?)
                        // Theoretically the target could be something different than the select element (in a more complex app at least),
                        // we may want to filter this out in such sugarized event handler
                        .target()
                        .unwrap_throw()
                        .dyn_ref::<web_sys::HtmlSelectElement>()
                        .unwrap_throw()
                        .value()
                        .parse()
                        .unwrap_throw();

                    // Try to keep the same beginning post
                    state.current_page = state.current_page * state.posts_per_page / count;

                    state.posts_per_page = count;
                }),
            ))
            .style(s("align-self", "center"));

            div((
                page_change_button("previous posts", BlogState::previous_page)
                    .disabled(*on_first_page),
                posts_per_page_select,
                page_change_button("next posts", BlogState::next_page),
            ))
            .style([s("display", "flex"), s("justify-content", "space-between")])
        },
    )
}

fn posts(state: &mut BlogState) -> impl Element<BlogState> {
    let start_post = state.current_page * state.posts_per_page;
    let post_range = (start_post)..(start_post + state.posts_per_page);
    div(post_range
        .map(|idx| {
            let idx = idx + 1;
            let title = h2(format!("Post {idx}"));
            let chat_elem = if idx % 2 == 0 {
                OneOf2::A(
                    div((
                        title,
                        p("Beautiful, possibly AI generated image:"),
                        img(()).src(format!("https://picsum.photos/300/200?random={idx}")),
                        p(LOREM_IPSUM),
                    ))
                    .style(s("background", "whitesmoke")),
                )
            } else {
                OneOf2::B(div((title, p(LOREM_IPSUM))))
            };

            // Note how this applies to either variants of the `OneOf` element, this would not be possible with type erasure via `AnyDomView`
            chat_elem.class("post")
        })
        .collect::<Vec<_>>())
    .class("posts")
}

fn app_logic(state: &mut BlogState) -> impl Element<BlogState> {
    div((
        h1("Blog"),
        controls(state).style(s("background", "whitesmoke")),
        posts(state),
    ))
}

pub fn main() {
    console_error_panic_hook::set_once();
    App::new(document_body(), BlogState::default(), app_logic).run();
}
