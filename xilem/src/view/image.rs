// Copyright 2024 the Xilem Authors
// SPDX-License-Identifier: Apache-2.0

use masonry::widget::{self, FillStrat};
use xilem_core::{Mut, ViewMarker};

use crate::{MessageResult, Pod, View, ViewCtx, ViewId};

/// Displays a bitmap Image.
///
/// By default, the Image will scale to fit its box constraints ([`FillStrat::Fill`]).
/// To configure this, call [`fill`](Image::fill) on the returned value.
pub fn image(image: vello::peniko::Image) -> Image {
    Image {
        image,
        fill: FillStrat::default(),
    }
}

/// The [`View`] created by [`image`].
///
/// See `image`'s docs for more details.
pub struct Image {
    image: vello::peniko::Image,
    fill: FillStrat,
}

impl Image {
    /// Specify the fill strategy.
    pub fn fill(mut self, fill: FillStrat) -> Self {
        self.fill = fill;
        self
    }
}

impl ViewMarker for Image {}
impl<State, Action> View<State, Action, ViewCtx> for Image {
    type Element = Pod<widget::Image>;
    type ViewState = ();

    fn build(&self, _: &mut ViewCtx) -> (Self::Element, Self::ViewState) {
        // Image's clone is cheap, so it's ok for this to be a view.
        (Pod::new(widget::Image::new(self.image.clone())), ())
    }

    fn rebuild<'el>(
        &self,
        prev: &Self,
        (): &mut Self::ViewState,
        _: &mut ViewCtx,
        mut element: Mut<'el, Self::Element>,
    ) -> Mut<'el, Self::Element> {
        if prev.fill != self.fill {
            element.set_fill_mode(self.fill);
        }
        if prev.image != self.image {
            element.set_image_data(self.image.clone());
        }
        element
    }

    fn teardown(&self, (): &mut Self::ViewState, _: &mut ViewCtx, _: Mut<'_, Self::Element>) {}

    fn message(
        &self,
        (): &mut Self::ViewState,
        _: &[ViewId],
        message: xilem_core::DynMessage,
        _: &mut State,
    ) -> MessageResult<Action> {
        tracing::error!("Message arrived in Label::message, but Label doesn't consume any messages, this is a bug");
        MessageResult::Stale(message)
    }
}
