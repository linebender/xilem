// Copyright 2025 the Xilem Authors
// SPDX-License-Identifier: Apache-2.0

use xilem::core::one_of::Either;
use xilem::core::{MessageProxy, NoElement, View};
use xilem::palette::css;
use xilem::style::{Gradient, Style};
use xilem::tokio::sync::mpsc::{UnboundedReceiver, UnboundedSender};

use xilem::view::{image, sized_box, spinner, worker};
use xilem::{Blob, Image, ImageFormat, ViewCtx, WidgetView, tokio};

use std::collections::HashMap;
use std::sync::Arc;

#[derive(Debug)]
struct AvatarRequest {
    avatar_url: String,
}

#[derive(Debug)]
struct AvatarResponse {
    url: String,
    image: Image,
}

#[derive(Default)]
pub(crate) struct Avatars {
    icons: HashMap<String, Option<Image>>,
    requester: Option<UnboundedSender<AvatarRequest>>,
    // TODO: lru: ...
}

impl Avatars {
    pub(crate) fn worker(&mut self) -> impl View<Self, (), ViewCtx, Element = NoElement> + use<> {
        worker(
            |proxy: MessageProxy<AvatarResponse>, mut rx: UnboundedReceiver<AvatarRequest>| async move {
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
                                tracing::warn!("Loading avatar from {url:?} failed: {err}.");
                            }
                        }
                    });
                }
            },
            |this: &mut Self, tx| {
                if this.requester.is_some() {
                    tracing::warn!("Unexpectedly got a second worker for requesting avatars.");
                }
                this.requester = Some(tx);
            },
            |this: &mut Self, response| {
                let ret = this.icons.insert(response.url, Some(response.image));
                if !matches!(ret, Some(None)) {
                    tracing::warn!("Potentially loaded or tried to load same avatar twice.");
                }
            },
        )
    }

    pub(crate) fn avatar<State: 'static>(
        &mut self,
        url: &str,
    ) -> impl WidgetView<State> + use<State> {
        if let Some(maybe_image) = self.icons.get(url) {
            if let Some(image_) = maybe_image {
                return Either::A(image(image_));
            }
        } else if let Some(requester) = self.requester.as_ref() {
            drop(requester.send(AvatarRequest {
                avatar_url: url.to_string(),
            }));
            self.icons.insert(url.to_string(), None);
        } else {
            // If the worker hasn't started yet, we have to wait until it does to do so.
        }
        Either::B(
            sized_box(spinner()).background_gradient(
                Gradient::new_linear(
                    // down-right
                    const { -45_f64.to_radians() },
                )
                .with_stops([css::YELLOW, css::LIME]),
            ),
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
