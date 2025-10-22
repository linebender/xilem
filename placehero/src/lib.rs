// Copyright 2025 the Xilem Authors
// SPDX-License-Identifier: Apache-2.0

// cargo rdme --workspace-project=placehero
// After editing the below, then check links in README.md

//! A mastodon client written in Xilem.
//!
//! We're assuming that all Mastodon servers supported are trusted (and so it's not a
//! privacy violation for them to know that two accounts you log in to are linked).
//! This link survives even if you log out of one and into the other, even in different sessions.
//! If this doesn't apply to you, we recommend not using Placehero.
//!
//! Features:
//!
//! - None

#![expect(clippy::todo, reason = "Landing intentionally in-progress work.")]

use std::sync::Arc;

use megalodon::entities::{Context, Instance, Status};
use megalodon::error::{Kind, OwnError};
use megalodon::{Megalodon, mastodon};
use xilem::core::one_of::{Either, OneOf, OneOf3, OneOf6};
use xilem::core::{NoElement, View, fork, lens, map_action, map_state};
use xilem::masonry::properties::types::AsUnit;
use xilem::tokio::sync::mpsc::{UnboundedReceiver, UnboundedSender};
use xilem::view::{
    CrossAxisAlignment, FlexExt, flex_col, flex_row, label, prose, sized_box, spinner, split,
    task_raw, text_button, text_input, worker_raw,
};
use xilem::winit::error::EventLoopError;
use xilem::{EventLoopBuilder, ViewCtx, WidgetView, WindowOptions, Xilem, tokio};

mod actions;
mod avatars;
mod components;
mod html_content;
mod login_flow;

pub(crate) use avatars::Avatars;
pub(crate) use html_content::status_html_to_plaintext;

use crate::actions::Navigation;
use crate::components::{Timeline, thread};
use crate::login_flow::PlaceheroWithLogin;

/// Our shared API client type.
///
/// Megalodon suggests using `dyn Megaldon`, but specifying Mastodon here specifically
/// has the advantage that go-to-definition works.
///
/// We also do not plan to support non-Mastodon servers at the moment.
/// However, keeping this type definition means a greater chance of a port to
/// outside Mastodon working.
#[expect(
    clippy::disallowed_types,
    reason = "We want to allow using the type only through this path"
)]
type Mastodon = Arc<mastodon::Mastodon>;

/// Execute the app in the given winit event loop.
pub fn run(event_loop: EventLoopBuilder) -> Result<(), EventLoopError> {
    Xilem::new_simple(
        MainState::Selecting,
        select_app,
        WindowOptions::new("Placehero: A placeholder named Mastodon client"),
    )
    .run_in(event_loop)
}

/// We are developing a version of Placehero which supports login.
///
/// This requires some quite gnarly refactors, so for now we built it "alongside"
/// the main Placehero.
#[expect(clippy::large_enum_variant, reason = "Not passed around.")]
enum MainState {
    Selecting,
    Old(Placehero),
    New(PlaceheroWithLogin),
}

fn select_app(state: &mut MainState) -> impl WidgetView<MainState> + use<> {
    match state {
        MainState::Selecting => OneOf3::A(
            flex_col((
                prose("Welcome to Placehero. This is an example of the Xilem GUI framework, which is a Mastodon client.\n\
                    We currently have decent support for browsing anonymously, and are currently developing our logged-in support in parallel to avoid regressions.")
                    .text_alignment(xilem::TextAlign::Center)
                    .flex(CrossAxisAlignment::Center),
                flex_row((
                    text_button("Browse Anonymously", |state: &mut MainState| {
                        *state = MainState::Old(Placehero::default());
                    }),
                    text_button("Log In", |state: &mut MainState| {
                        *state = MainState::New(PlaceheroWithLogin::new());
                    }),
                ))
                .main_axis_alignment(xilem::view::MainAxisAlignment::Center),
            ))
            .main_axis_alignment(xilem::view::MainAxisAlignment::Center),
        ),
        MainState::Old(_) => OneOf::B(lens(app_logic, |state| {
            let MainState::Old(placehero) = state else {
                unreachable!()
            };
            placehero
        })),
        MainState::New(_) => OneOf::C(lens(login_flow::app_logic, |state| {
            let MainState::New(placehero) = state else {
                unreachable!()
            };
            placehero
        })),
    }
}

struct Placehero {
    mastodon: Mastodon,
    instance: Option<Instance>,
    timeline: Option<Timeline>,
    show_context: Option<Status>,
    context: Option<Context>,
    context_sender: Option<UnboundedSender<String>>,
    account_sender: Option<UnboundedSender<String>>,
    timeline_box_contents: String,
    loading_timeline: bool,
    not_found_acct: Option<String>,
}

impl Default for Placehero {
    fn default() -> Self {
        // TODO: Configurable server?
        let base_url = "https://mastodon.online".to_string();
        // TODO: Determine what user agent we want to send.
        // Currently we send "megalodon", as that is the default in the library.
        let user_agent = None;

        #[expect(
            clippy::disallowed_types,
            reason = "We are constructing a value of the type, which we will never directly use elsewhere"
        )]
        let mastodon = mastodon::Mastodon::new(base_url, None, user_agent)
            .expect("Provided User Agent is valid");

        Self {
            mastodon: Arc::new(mastodon),
            instance: None,
            timeline: None,
            show_context: None,
            context: None,
            context_sender: None,
            account_sender: None,
            timeline_box_contents: "raph".to_string(),
            loading_timeline: false,
            not_found_acct: None,
        }
    }
}

impl Placehero {
    fn sidebar(&mut self) -> impl WidgetView<Self, Navigation> + use<> {
        if let Some(instance) = &self.instance {
            let back = if self.show_context.is_some() {
                // TODO: Make the ⬅️ arrow not be available to screen readers.
                Either::A(text_button("⬅️ Back to Timeline", |_: &mut Self| {
                    Navigation::Home
                }))
            } else {
                // Sized box of a flex because nested flexes aren't supported (? is this true)
                // TODO: Ideally, we'd be able to use Either with fragments here
                Either::B(sized_box(flex_col((
                    text_input(
                        self.timeline_box_contents.clone(),
                        |state: &mut Self, string| {
                            state.timeline_box_contents = string;
                            Navigation::None
                        },
                    )
                    .on_enter(|_, user| Navigation::LoadUser(user))
                    .disabled(self.loading_timeline),
                    self.loading_timeline
                        .then(|| sized_box(spinner()).width(50.px()).height(50.px())),
                    text_button("Go", |state: &mut Self| {
                        Navigation::LoadUser(state.timeline_box_contents.clone())
                    }),
                ))))
            };
            Either::A(flex_col((
                label("Connected to:"),
                // TODO: We should probably use an ArcStr for this?
                prose(instance.title.as_str()),
                back,
            )))
        } else {
            Either::B(prose("Not yet connected (or other unhandled error)"))
        }
    }

    fn main_view(&mut self) -> impl WidgetView<Self, Navigation> + use<> {
        if let Some(show_context) = self.show_context.as_ref() {
            if let Some(context) = self.context.as_ref() {
                // TODO: Display the status until the entire thread loads; this is hard because
                // the thread's scroll position would jump.
                OneOf6::A(thread(show_context, context))
            } else {
                OneOf::B(prose("Loading thread"))
            }
        } else if self.loading_timeline {
            // Hack: Flex allows the sized box to not take up the full size.
            OneOf::C(flex_col(
                sized_box(spinner()).width(50.px()).height(50.px()),
            ))
        } else if let Some(acct) = self.not_found_acct.as_ref() {
            OneOf::D(prose(format!(
                "Could not find account @{acct} on this server. \
                 You might need to include the server name of the account, if it's on a different server."
            )))
        } else if let Some(timline) = self.timeline.as_mut() {
            OneOf::E(map_state(
                timline.view(self.mastodon.clone()),
                // In the current edition of the app, the timeline is never removed
                // If it ever is, we'll need to be more careful here.
                // The patterns are still in flux.
                |this: &mut Self| this.timeline.as_mut().unwrap(),
            ))
        } else {
            OneOf::F(prose("No statuses yet loaded"))
        }
    }
}

fn app_logic(app_state: &mut Placehero) -> impl WidgetView<Placehero> + use<> {
    Avatars::provide(fork(
        map_action(
            split(app_state.sidebar(), app_state.main_view()).split_point(0.2),
            |state, action| match action {
                Navigation::LoadContext(status) => {
                    state
                        .context_sender
                        .as_ref()
                        .unwrap()
                        .send(status.id.clone())
                        .unwrap();
                    state.show_context = Some(status);
                    state.context = None;
                }
                Navigation::LoadUser(user) => {
                    // It's fine to set `timeline_box_contents` manually here, because we
                    // set state.loading_timeline (i.e. the box will become disabled).
                    state.timeline_box_contents = user.clone();
                    state.account_sender.as_ref().unwrap().send(user).unwrap();
                    // In theory, we should set the Timeline to None here. However,
                    // it currently runs into `teardown` requiring the state to be available
                    // state.timeline = None;
                    state.loading_timeline = true;
                    state.context = None;
                    state.show_context = None;
                }
                Navigation::Home => {
                    state.context = None;
                    state.show_context = None;
                }
                Navigation::None => {}
            },
        ),
        (
            load_instance(app_state.mastodon.clone()),
            load_account(app_state.mastodon.clone()),
            load_contexts(app_state.mastodon.clone()),
        ),
    ))
}

fn load_contexts(
    mastodon: Mastodon,
) -> impl View<Placehero, (), ViewCtx, Element = NoElement> + use<> {
    worker_raw(
        move |result, mut recv: UnboundedReceiver<String>| {
            let mastodon = mastodon.clone();
            async move {
                while let Some(req) = recv.recv().await {
                    let mastodon = mastodon.clone();
                    let result = result.clone();
                    // TODO: Cancel the previous task?
                    // There is probably some `select!`-adjacent helper for this?
                    tokio::task::spawn(async move {
                        // Note that error handling is deferred to the on_response handler
                        let context_result = mastodon.get_status_context(req.clone(), None).await;
                        // We choose not to handle the case where the event loop has ended
                        drop(result.message((context_result, req)));
                    });
                }
            }
        },
        |app_state: &mut Placehero, sender| app_state.context_sender = Some(sender),
        |app_state: &mut Placehero, (event, id)| match event {
            Ok(context)
                if app_state
                    .show_context
                    .as_ref()
                    .is_some_and(|it| it.id == id) =>
            {
                app_state.context = Some(context.json);
            }
            Ok(_) => {
                tracing::warn!("Dropping context whose request was superseded.");
            }
            Err(megalodon::error::Error::RequestError(e)) if e.is_connect() => {
                todo!()
            }
            Err(e) => {
                todo!("handle {e}")
            }
        },
    )
}

fn load_instance(
    mastodon: Mastodon,
) -> impl View<Placehero, (), ViewCtx, Element = NoElement> + use<> {
    task_raw(
        move |result| {
            let mastodon = mastodon.clone();
            async move {
                // We choose not to handle the case where the event loop has ended
                let instance_result = mastodon.get_instance().await;
                // Note that error handling is deferred to the on_event handler
                drop(result.message(instance_result));
            }
        },
        |app_state: &mut Placehero, event| match event {
            Ok(instance) => app_state.instance = Some(instance.json),
            Err(megalodon::error::Error::RequestError(e)) if e.is_connect() => {
                todo!()
            }
            Err(e) => {
                todo!("handle {e}")
            }
        },
    )
}

fn load_account(
    mastodon: Mastodon,
) -> impl View<Placehero, (), ViewCtx, Element = NoElement> + use<> {
    worker_raw(
        move |result, mut recv: UnboundedReceiver<String>| {
            let mastodon = mastodon.clone();
            async move {
                while let Some(req) = recv.recv().await {
                    let instance_result = mastodon.lookup_account(req.clone()).await;
                    // We choose not to handle the case where the event loop has ended
                    // Note that error handling is deferred to the on_event handler
                    drop(result.message((instance_result, req)));
                }
            }
        },
        |app_state: &mut Placehero, sender| app_state.account_sender = Some(sender),
        |app_state: &mut Placehero, (event, acct)| match event {
            Ok(instance) => {
                app_state.timeline = Some(Timeline::new_for_account(instance.json));
                app_state.loading_timeline = false;
                app_state.not_found_acct = None;
            }
            Err(megalodon::error::Error::RequestError(e)) if e.is_connect() => {
                todo!()
            }
            Err(megalodon::error::Error::OwnError(
                e @ OwnError {
                    kind: Kind::HTTPStatusError,
                    ..
                },
            )) => {
                tracing::error!("Failure to to load account: {e}.");
                // TODO: Handle more gracefully/surface to the user.
                // This at least lets the user retry.
                // Note that we don't unset the timeline here, because it's technically
                // possible for this response to arrive extremely quickly
                app_state.loading_timeline = false;
                app_state.not_found_acct = Some(acct);
            }
            Err(e) => {
                todo!("handle {e}")
            }
        },
    )
}
