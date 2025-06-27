// Copyright 2025 the Xilem Authors
// SPDX-License-Identifier: Apache-2.0

//! A mastodon client written in Xilem.
//!
//! Features:
//!
//! - None

#![expect(clippy::todo, reason = "Landing intentionally in-progress work.")]

use std::sync::Arc;

use components::timeline;
use megalodon::entities::{Account, Context, Instance, Status};
use megalodon::megalodon::GetAccountStatusesInputOptions;
use megalodon::{Megalodon, mastodon};
use xilem::core::one_of::{Either, OneOf, OneOf4};
use xilem::core::{NoElement, View, fork, lens};
use xilem::tokio::sync::mpsc::{UnboundedReceiver, UnboundedSender};
use xilem::view::{button, flex, label, prose, split, task_raw, worker_raw};
use xilem::winit::error::EventLoopError;
use xilem::{EventLoopBuilder, ViewCtx, WidgetView, WindowOptions, Xilem, tokio};

mod avatars;
mod components;
mod html_content;

pub(crate) use avatars::Avatars;
pub(crate) use html_content::status_html_to_plaintext;

use crate::components::thread;

/// Our shared API client type.
///
/// Megalodon suggests using `dyn Megaldon`, but specifying Mastodon here specifically
/// has the advantage that go-to-definition works.
///
/// We also do not plan to support non-Mastodon servers at the moment.
/// However, keeping this type definition means a greater chance of.
#[expect(
    clippy::disallowed_types,
    reason = "We want to allow using the type only through this path"
)]
type Mastodon = Arc<mastodon::Mastodon>;

struct Placehero {
    mastodon: Mastodon,
    instance: Option<Instance>,
    thread_statuses: Vec<Status>,
    account: Option<Account>,
    avatars: Avatars,
    show_context: Option<Status>,
    context: Option<Context>,
    context_sender: Option<UnboundedSender<String>>,
}

/// Execute the app in the given winit event loop.
pub fn run(event_loop: EventLoopBuilder) -> Result<(), EventLoopError> {
    let base_url = "https://mastodon.online".to_string();
    // TODO: Determine what user agent we want to send.
    // Currently we send "megalodon", as that is the default in the library.
    let user_agent = None;

    #[expect(
        clippy::disallowed_types,
        reason = "We are constructing a value of the type, which we will never directly use elsewhere"
    )]
    let mastodon =
        mastodon::Mastodon::new(base_url, None, user_agent).expect("Provided User Agent is valid");

    let app_state = Placehero {
        mastodon: Arc::new(mastodon),
        instance: None,
        account: None,
        thread_statuses: Vec::new(),
        avatars: Avatars::default(),
        show_context: None,
        context: None,
        context_sender: None,
    };

    Xilem::new_simple(
        app_state,
        app_logic,
        WindowOptions::new("Placehero: A placeholder named Mastodon client"),
    )
    .run_in(event_loop)
}

impl Placehero {
    fn sidebar(&mut self) -> impl WidgetView<Self> + use<> {
        if let Some(instance) = &self.instance {
            let back = if self.show_context.is_some() {
                Some(button("🔙", |app_state: &mut Self| {
                    app_state.show_context = None;
                    app_state.context = None;
                }))
            } else {
                None
            };
            Either::A(flex((
                label("Connected to:"),
                // TODO: We should probably use an ArcStr for this?
                prose(instance.title.as_str()),
                back,
            )))
        } else {
            Either::B(prose("Not yet connected (or other unhandled error)"))
        }
    }

    fn main_view(&mut self) -> impl WidgetView<Self> + use<> {
        if let Some(show_context) = self.show_context.as_ref() {
            if let Some(context) = self.context.as_ref() {
                // TODO: Display the status until the entire thread loads; this is hard because
                // the thread's scroll position would jump.
                OneOf4::A(thread(&mut self.avatars, show_context, context))
            } else {
                OneOf::B(prose("Loading thread"))
            }
        } else if !self.thread_statuses.is_empty() {
            OneOf::C(timeline(&mut self.thread_statuses, &mut self.avatars))
        } else {
            OneOf::D(prose("No statuses yet loaded"))
        }
    }
}

fn app_logic(app_state: &mut Placehero) -> impl WidgetView<Placehero> + use<> {
    fork(
        split(app_state.sidebar(), app_state.main_view()).split_point(0.2),
        (
            load_instance(app_state.mastodon.clone()),
            load_account(app_state.mastodon.clone()),
            load_contexts(app_state.mastodon.clone()),
            app_state
                .account
                .as_ref()
                .map(|it| it.id.clone())
                .map(|id| load_statuses(app_state.mastodon.clone(), id)),
            lens(
                |avatars: &mut Avatars| avatars.worker(),
                |app_state: &mut Placehero| &mut app_state.avatars,
            ),
        ),
    )
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
            Err(megalodon::error::Error::RequestError(e)) if e.is_status() => {
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
            Err(megalodon::error::Error::RequestError(e)) if e.is_status() => {
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
    task_raw(
        move |result| {
            let mastodon = mastodon.clone();
            async move {
                // We choose not to handle the case where the event loop has ended
                let instance_result = mastodon.lookup_account("raph".to_string()).await;
                // Note that error handling is deferred to the on_event handler
                drop(result.message(instance_result));
            }
        },
        |app_state: &mut Placehero, event| match event {
            Ok(instance) => app_state.account = Some(instance.json),
            Err(megalodon::error::Error::RequestError(e)) if e.is_connect() => {
                todo!()
            }
            Err(megalodon::error::Error::RequestError(e)) if e.is_status() => {
                todo!()
            }
            Err(e) => {
                todo!("handle {e}")
            }
        },
    )
}

fn load_statuses(
    mastodon: Mastodon,
    id: String,
) -> impl View<Placehero, (), ViewCtx, Element = NoElement> + use<> {
    task_raw(
        move |result| {
            let mastodon = mastodon.clone();
            let id = id.clone();
            async move {
                // We choose not to handle the case where the event loop has ended
                let instance_result = mastodon
                    .get_account_statuses(
                        id,
                        Some(&GetAccountStatusesInputOptions {
                            exclude_reblogs: Some(false),
                            exclude_replies: Some(true),
                            ..Default::default()
                        }),
                    )
                    .await;
                // Note that error handling is deferred to the on_event handler
                drop(result.message(instance_result));
            }
        },
        |app_state: &mut Placehero, event| match event {
            Ok(instance) => app_state.thread_statuses = instance.json,
            Err(megalodon::error::Error::RequestError(e)) if e.is_connect() => {
                todo!()
            }
            Err(megalodon::error::Error::RequestError(e)) if e.is_status() => {
                todo!()
            }
            Err(e) => {
                todo!("handle {e}")
            }
        },
    )
}
