// Copyright 2025 the Xilem Authors
// SPDX-License-Identifier: Apache-2.0

//! A mastodon client written in Xilem.
//!
//! Features:
//!
//! - None

#![expect(clippy::todo, reason = "Landing intentionally in-progress work.")]

use std::sync::Arc;

use megalodon::{Megalodon, entities::Instance, mastodon};
use xilem::{
    EventLoopBuilder, WidgetView, WindowOptions, Xilem,
    core::{
        fork,
        one_of::{Either, OneOf},
    },
    view::{flex, label, prose, task_raw},
    winit::error::EventLoopError,
};

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
            OneOf::B(label("Not yet connected (or other unhandled error)"))
        }
    }
}

fn app_logic(app_state: &mut Placehero) -> impl WidgetView<Placehero> + use<> {
    let mastodon = app_state.mastodon.clone();
    fork(
        app_state.sidebar(),
        task_raw(
            move |result| {
                let megalodon = mastodon.clone();
                async move {
                    // We choose not to handle the case where the event loop has ended
                    let instance_result = megalodon.get_instance().await;
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
        ),
    )
}

/// Execute the app in the given winit event loop.
pub fn run(event_loop: EventLoopBuilder) -> Result<(), EventLoopError> {
    let base_url = "https://mastodon.online".to_string();
    // TODO: Determine what user agent we want to send.
    // Currently we send "megalodon", as that is the default in the library
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
    };

    Xilem::new_simple(
        app_state,
        app_logic,
        WindowOptions::new("Placehero: A placeholder named Mastodon client"),
    )
    .run_in(event_loop)
}
