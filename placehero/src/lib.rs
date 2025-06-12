// Copyright 2025 the Xilem Authors
// SPDX-License-Identifier: Apache-2.0

//! A mastodon client written in Xilem.
//!
//! Features:
//!
//! - None

#![expect(clippy::todo, reason = "Landing intentionally in-progress work.")]

use std::sync::Arc;

use megalodon::{Megalodon, entities::Instance};
use xilem::{
    EventLoopBuilder, WidgetView, WindowOptions, Xilem,
    core::{
        fork,
        one_of::{Either, OneOf},
    },
    view::{flex, label, prose, task_raw},
    winit::error::EventLoopError,
};

struct Placehero {
    megalodon: Arc<dyn Megalodon + Send + Sync + 'static>,
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
    let megalodon = app_state.megalodon.clone();
    fork(
        app_state.sidebar(),
        task_raw(
            move |result| {
                let megalodon = megalodon.clone();
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
    let megalodon = megalodon::generator(
        megalodon::SNS::Mastodon,
        "https://mastodon.online".to_string(),
        None,
        Some("Placehero".into()),
    )
    // TODO: Better error handling
    .unwrap();
    let app_state = Placehero {
        megalodon: megalodon.into(),
        instance: None,
    };

    Xilem::new_simple(
        app_state,
        app_logic,
        WindowOptions::new("Placehero: A placeholder named Mastodon client"),
    )
    .run_in(event_loop)
}
