// Copyright 2024 the Xilem Authors
// SPDX-License-Identifier: Apache-2.0

//! The bitmap image widget.

use masonry::widgets;
use vello::peniko::ImageBrush;

use crate::core::{Arg, MessageCtx, MessageResult, Mut, View, ViewArgument, ViewMarker};
use crate::view::Prop;
use crate::{Pod, ViewCtx, WidgetView};

pub use masonry::properties::ObjectFit;

/// Displays the bitmap `image`.
///
/// By default, the Image will scale to fit its box constraints ([`ObjectFit::Fill`]).
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
    }
}

/// The [`View`] created by [`image`].
///
/// See `image`'s docs for more details.
#[must_use = "View values do nothing unless provided to Xilem."]
pub struct Image {
    image: ImageBrush,
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
}

impl ViewMarker for Image {}
impl<State: ViewArgument, Action> View<State, Action, ViewCtx> for Image {
    type Element = Pod<widgets::Image>;
    type ViewState = ();

    fn build(&self, ctx: &mut ViewCtx, _: Arg<'_, State>) -> (Self::Element, Self::ViewState) {
        (ctx.create_pod(widgets::Image::new(self.image.clone())), ())
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
