// Copyright 2024 the Xilem Authors
// SPDX-License-Identifier: Apache-2.0

//! The bitmap image widget.

use masonry::core::ArcStr;
use masonry::widgets;
use vello::peniko::ImageBrush;

use crate::core::{Arg, MessageCtx, MessageResult, Mut, View, ViewArgument, ViewMarker};
use crate::view::Prop;
use crate::{Pod, ViewCtx, WidgetView};

pub use masonry::properties::ObjectFit;

/// Displays the bitmap `image`.
///
/// By default, the Image will be scaled to fully fit within the container ([`ObjectFit::Contain`]).
/// To configure this, call [`fit`](Image::fit) on the returned value.
///
/// Corresponds to the [`Image`](widgets::Image) widget.
///
/// It is not currently supported to use a GPU-resident [texture](vello::wgpu::Texture) in this widget.
/// See [#vello > vello adding wgpu texture buffers to scene](https://xi.zulipchat.com/#narrow/channel/197075-vello/topic/vello.20adding.20wgpu.20texture.20buffers.20to.20scene/with/456486490)
/// for discussion.
pub fn image(image: impl Into<ImageBrush>) -> Image {
    Image {
        image: image.into(),
        decorative: false,
        alt_text: None,
    }
}

/// The [`View`] created by [`image`].
///
/// See `image`'s docs for more details.
#[must_use = "View values do nothing unless provided to Xilem."]
pub struct Image {
    image: ImageBrush,
    decorative: bool,
    alt_text: Option<ArcStr>,
}

impl Image {
    // Because this method is image-specific, we don't add it to the Style trait.
    /// Specify the object fit.
    pub fn fit<State: ViewArgument, Action: 'static>(
        self,
        fill: ObjectFit,
    ) -> Prop<ObjectFit, Self, State, Action> {
        self.prop(fill)
    }

    /// Specifies whether the image is decorative, meaning it doesn't have meaningful content and is only for visual presentation.
    ///
    /// If `is_decorative` is `true`, the image will be ignored by screen readers.
    pub fn decorative(mut self, is_decorative: bool) -> Self {
        self.decorative = is_decorative;
        self
    }

    /// Set the text that will describe the image to screen readers.
    ///
    /// Users are encouraged to set alt text for the image.
    /// If possible, the alt-text should succinctly describe what the image represents.
    ///
    /// If the image is decorative users should set alt text to `""`.
    /// If it's too hard to describe through text, the alt text should be left unset.
    /// This allows accessibility clients to know that there is no accessible description of the image content.
    pub fn alt_text(mut self, alt_text: impl Into<ArcStr>) -> Self {
        self.alt_text = Some(alt_text.into());
        self
    }
}
impl ViewMarker for Image {}
impl<State: ViewArgument, Action> View<State, Action, ViewCtx> for Image {
    type Element = Pod<widgets::Image>;
    type ViewState = ();

    fn build(&self, ctx: &mut ViewCtx, _: Arg<'_, State>) -> (Self::Element, Self::ViewState) {
        let mut image = widgets::Image::new(self.image.clone()).decorative(self.decorative);
        if let Some(alt_text) = &self.alt_text {
            image = image.with_alt_text(alt_text.clone());
        }
        (ctx.create_pod(image), ())
    }

    fn rebuild(
        &self,
        prev: &Self,
        (): &mut Self::ViewState,
        _: &mut ViewCtx,
        mut element: Mut<'_, Self::Element>,
        _: Arg<'_, State>,
    ) {
        if prev.image != self.image {
            widgets::Image::set_image_data(&mut element, self.image.clone());
        }
        if self.decorative != prev.decorative {
            widgets::Image::set_decorative(&mut element, self.decorative);
        }
        if self.alt_text != prev.alt_text {
            widgets::Image::set_alt_text(&mut element, self.alt_text.clone());
        }
    }

    fn teardown(&self, (): &mut Self::ViewState, _: &mut ViewCtx, _: Mut<'_, Self::Element>) {}

    fn message(
        &self,
        (): &mut Self::ViewState,
        message: &mut MessageCtx,
        _: Mut<'_, Self::Element>,
        _: Arg<'_, State>,
    ) -> MessageResult<Action> {
        tracing::error!(
            ?message,
            "Message arrived in Image::message, but Image doesn't consume any messages, this is a bug."
        );
        MessageResult::Stale
    }
}
