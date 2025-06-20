// Copyright 2025 the Xilem Authors
// SPDX-License-Identifier: Apache-2.0

//! A mastodon client written in Xilem.
//!
//! Features:
//!
//! - None

#![expect(clippy::todo, reason = "Landing intentionally in-progress work.")]

use std::sync::Arc;

use megalodon::{
    Megalodon,
    entities::{Account, Instance, Status},
    mastodon,
    megalodon::GetAccountStatusesInputOptions,
};
use xilem::{
    EventLoopBuilder, ViewCtx, WidgetView, WindowOptions, Xilem,
    core::{NoElement, View, fork, lens, one_of::Either},
    palette::css,
    style::Style,
    view::{GridExt, GridParams, flex, grid, label, portal, prose, sized_box, split, task_raw},
    winit::error::EventLoopError,
};

use crate::avatars::Avatars;
use crate::html_content::status_html_to_plaintext;

mod avatars;
mod html_content;

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
    statuses: Vec<Status>,
    account: Option<Account>,
    avatars: Avatars,
}

impl Placehero {
    fn sidebar(&mut self) -> impl WidgetView<Self> + use<> {
        if let Some(instance) = &self.instance {
            Either::A(flex((
                label("Connected to:"),
                // TODO: We should probably use an ArcStr for this?
                prose(instance.title.as_str()),
            )))
        } else {
            Either::B(prose("Not yet connected (or other unhandled error)"))
        }
    }

    fn main_view(&mut self) -> impl WidgetView<Self> + use<> {
        if self.statuses.is_empty() {
            Either::A(prose("No statuses yet loaded"))
        } else {
            Either::B(portal(flex(
                self.statuses
                    .iter()
                    .map(|status| status_view(&mut self.avatars, status))
                    .collect::<Vec<_>>(),
            )))
        }
    }
}

fn status_view(avatars: &mut Avatars, status: &Status) -> impl WidgetView<Placehero> + use<> {
    sized_box(grid(
        (
            avatars.avatar(&status.account.avatar_static).grid_pos(0, 0),
            prose(status.account.display_name.as_str()).grid_pos(1, 0),
            prose(status.account.username.as_str()).grid_pos(2, 0),
            prose(status_html_to_plaintext(status.content.as_str()))
                .grid_item(GridParams::new(0, 1, 3, 1)),
            prose(status.created_at.to_rfc2822()).grid_pos(0, 2),
            prose(status.favourites_count.to_string()).grid_pos(1, 2),
            prose(status.replies_count.to_string()).grid_pos(2, 2),
        ),
        3,
        3,
    ))
    .expand_width()
    .height(300.0)
    .border(css::WHITE, 2.)
}

fn app_logic(app_state: &mut Placehero) -> impl WidgetView<Placehero> + use<> {
    fork(
        split(app_state.sidebar(), app_state.main_view()),
        (
            load_instance(app_state.mastodon.clone()),
            load_account(app_state.mastodon.clone()),
            app_state
                .account
                .as_ref()
                .map(|it| it.id.clone())
                .map(|id| load_statuses(app_state.mastodon.clone(), id)),
            lens(
                |avatars: &mut Avatars| avatars.worker(),
                app_state,
                |app_state: &mut Placehero| &mut app_state.avatars,
            ),
        ),
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
                            exclude_reblogs: Some(true),
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
            Ok(instance) => app_state.statuses = instance.json,
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
        statuses: Vec::new(),
        avatars: Avatars::default(),
    };

    Xilem::new_simple(
        app_state,
        app_logic,
        WindowOptions::new("Placehero: A placeholder named Mastodon client"),
    )
    .run_in(event_loop)
}
