// Copyright 2025 the Xilem Authors
// SPDX-License-Identifier: Apache-2.0

use std::collections::HashMap;
use std::sync::Arc;

use xilem::core::one_of::Either;
use xilem::core::{
    MessageProxy, MessageResult, NoElement, Resource, View, fork, map_message,
    on_action_with_context, provides, with_context,
};
use xilem::masonry::properties::types::AsUnit;
use xilem::palette::css;
use xilem::style::{Gradient, Style};
use xilem::tokio::sync::mpsc::{UnboundedReceiver, UnboundedSender};
use xilem::view::{env_worker, image, sized_box, spinner};
use xilem::{Blob, Image, ImageFormat, ViewCtx, WidgetView, tokio};

#[derive(Debug)]
struct AvatarRequest {
    avatar_url: String,
}

#[derive(Debug)]
struct AvatarResponse {
    url: String,
    image: Image,
}

#[derive(Debug)]
pub(crate) struct Avatars {
    icons: HashMap<String, Option<Image>>,
    requester: Option<UnboundedSender<AvatarRequest>>,
    // TODO: lru: ...
}

impl Resource for Avatars {}

impl Avatars {
    /// Get the avatar view/component for the given url, as a 50x50 pixel box.
    ///
    /// This will fetch the image data from the URL, and cache it.
    /// If the image hasn't yet loaded, will show a placeholder,
    ///
    ///  Requires that this View is within a [`Self::provide`] call.
    // TODO: ArcStr for URL?
    pub(crate) fn avatar<State: 'static>(url: String) -> impl WidgetView<State> + use<State> {
        with_context(move |this: &mut Self, _: &mut State| {
            let width = 50.px();
            let height = 50.px();
            if let Some(maybe_image) = this.icons.get(&url) {
                if let Some(image_) = maybe_image {
                    return Either::A(sized_box(image(image_)).width(width).height(height));
                }
            } else if let Some(requester) = this.requester.as_ref() {
                drop(requester.send(AvatarRequest {
                    avatar_url: url.to_string(),
                }));
                this.icons.insert(url.to_string(), None);
            } else {
                // If the worker hasn't started yet, we have to wait until it does to do so.
            }
            Either::B(
                sized_box(spinner().color(css::BLACK))
                    .background_gradient(
                        Gradient::new_linear(
                            // down-right
                            const { -45_f64.to_radians() },
                        )
                        .with_stops([css::YELLOW, css::LIME]),
                    )
                    .width(width)
                    .height(height)
                    .padding(4.0),
            )
        })
    }

    /// Provide support for Mastodon Avatar display to the `child` view.
    pub(crate) fn provide<State, Action, Child>(
        child: Child,
    ) -> impl WidgetView<State, Action, Element = Child::Element>
    where
        Child: WidgetView<State, Action>,
        State: 'static,
        Action: 'static,
    {
        provides(
            |_| Self {
                icons: HashMap::default(),
                requester: None,
            },
            fork(child, Self::worker()),
        )
    }

    fn worker<State, Action>()
    -> impl View<State, Action, ViewCtx, Element = NoElement> + use<State, Action>
    where
        State: 'static,
        Action: 'static,
    {
        map_message(
            on_action_with_context(
                |_: &mut State, this: &mut Self, response| {
                    let ret = this.icons.insert(response.url, Some(response.image));
                    if !matches!(ret, Some(None)) {
                        tracing::warn!("Potentially loaded or tried to load same avatar twice.");
                    }
                },
                env_worker(
                    |proxy: MessageProxy<AvatarResponse>,
                     mut rx: UnboundedReceiver<AvatarRequest>| async move {
                        while let Some(url) = rx.recv().await {
                            let proxy = proxy.clone();
                            tokio::task::spawn(async move {
                                let url = url.avatar_url;
                                let result = image_from_url(&url).await;
                                match result {
                                    // We choose not to handle the case where the event loop has ended
                                    Ok(image) => drop(proxy.message(AvatarResponse { url, image })),
                                    // TODO: Report in the frontend?
                                    Err(err) => {
                                        tracing::warn!(
                                            "Loading avatar from {url:?} failed: {err}."
                                        );
                                    }
                                }
                            });
                        }
                    },
                    |_: &mut State, this: &mut Self, tx| {
                        if this.requester.is_some() {
                            tracing::warn!(
                                "Unexpectedly got a second worker for requesting avatars."
                            );
                        }
                        this.requester = Some(tx);
                    },
                    |_: &mut State, response| response,
                ),
            ),
            // Convert to the user-defined action type by ignoring any action we actually submit.
            |_, action| match action {
                MessageResult::Action(_) => MessageResult::RequestRebuild,
                MessageResult::RequestRebuild => MessageResult::RequestRebuild,
                MessageResult::Nop => MessageResult::Nop,
                MessageResult::Stale => MessageResult::Stale,
            },
        )
    }
}

/// Load an [`Image`] from the given url.
///
/// N.B. This is functionality shared with `http_cats`
pub(crate) async fn image_from_url(url: &str) -> anyhow::Result<Image> {
    let response = reqwest::get(url).await?;
    let bytes = response.bytes().await?;
    let image = image::load_from_memory(&bytes)?.into_rgba8();
    let width = image.width();
    let height = image.height();
    let data = image.into_vec();
    Ok(Image::new(
        Blob::new(Arc::new(data)),
        ImageFormat::Rgba8,
        width,
        height,
    ))
}
