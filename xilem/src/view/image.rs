// Copyright 2024 the Xilem Authors
// SPDX-License-Identifier: Apache-2.0

//! The bitmap image widget.

use masonry::widgets::{self};

use crate::core::{DynMessage, Mut, ViewMarker};
use crate::{MessageResult, Pod, View, ViewCtx, ViewId};

pub use masonry::core::ObjectFit;
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
pub fn image(image: &vello::peniko::Image) -> Image {
    Image {
        // Image only contains a `Blob` and Copy fields, and so is cheap to clone.
        // We take by reference as we expect all users of this API will need to clone, and it's
        // easier than documenting that cloning is cheap.
        image: image.clone(),
        object_fit: ObjectFit::default(),
    }
}

/// The [`View`] created by [`image`].
///
/// See `image`'s docs for more details.
#[must_use = "View values do nothing unless provided to Xilem."]
pub struct Image {
    image: vello::peniko::Image,
    object_fit: ObjectFit,
}

impl Image {
    /// Specify the object fit.
    pub fn fit(mut self, fill: ObjectFit) -> Self {
        self.object_fit = fill;
        self
    }
}

impl ViewMarker for Image {}
impl<State, Action> View<State, Action, ViewCtx> for Image {
    type Element = Pod<widgets::Image>;
    type ViewState = ();

    fn build(&self, ctx: &mut ViewCtx) -> (Self::Element, Self::ViewState) {
        let pod = ctx.new_pod(widgets::Image::new(self.image.clone()));
        (pod, ())
    }

    fn rebuild(
        &self,
        prev: &Self,
        (): &mut Self::ViewState,
        _: &mut ViewCtx,
        mut element: Mut<Self::Element>,
    ) {
        if prev.object_fit != self.object_fit {
            widgets::Image::set_fit_mode(&mut element, self.object_fit);
        }
        if prev.image != self.image {
            widgets::Image::set_image_data(&mut element, self.image.clone());
        }
    }

    fn teardown(&self, (): &mut Self::ViewState, _: &mut ViewCtx, _: Mut<Self::Element>) {}

    fn message(
        &self,
        (): &mut Self::ViewState,
        _: &[ViewId],
        message: DynMessage,
        _: &mut State,
    ) -> MessageResult<Action> {
        tracing::error!(
            "Message arrived in Image::message, but Image doesn't consume any messages, this is a bug"
        );
        MessageResult::Stale(message)
    }
}
