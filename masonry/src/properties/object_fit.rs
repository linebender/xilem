// Copyright 2025 the Xilem Authors
// SPDX-License-Identifier: Apache-2.0

use std::any::TypeId;

use masonry_core::core::{Property, UpdateCtx};
use vello::kurbo::{Affine, Size};

// These are based on https://developer.mozilla.org/en-US/docs/Web/CSS/object-fit
/// Strategies for inscribing a rectangle inside another rectangle.
///
/// Default value is [`Self::Contain`].
#[derive(Clone, Copy, Debug, PartialEq)]
pub enum ObjectFit {
    /// The content is scaled to fully fit within the container.
    ///
    /// The content's aspect ratio is maintained.
    ///
    /// If the content's aspect ratio does not match the aspect ratio of its container,
    /// then the content will not cover the whole container and nothing will overflow.
    Contain,
    /// The content is scaled to fully fill the container.
    ///
    /// The content's aspect ratio is maintained.
    ///
    /// If the content's aspect ratio does not match the aspect ratio of its container,
    /// then the content will overflow the container.
    Cover,
    /// The content is stretched to fully fill the container.
    ///
    /// If the content's aspect ratio does not match the aspect ratio of its container,
    /// then the content will be stretched to fit exactly, changing its aspect ratio.
    Fill,
    /// The content is scaled to fully fill the container's height.
    ///
    /// The content's aspect ratio is maintained.
    ///
    /// This may result in letterboxed or overflowing width.
    FitHeight,
    /// The content is scaled to fully fill the container's width.
    ///
    /// The content's aspect ratio is maintained.
    ///
    /// This may result in letterboxed or overflowing height.
    FitWidth,
    /// The content's size is not changed at all.
    None,
    /// The content is only scaled down.
    ///
    /// This behaves as a mix of [`None`] and [`Contain`], resulting in whichever variant
    /// gives the smaller size.
    ///
    /// [`None`]: ObjectFit::None
    /// [`Contain`]: ObjectFit::Contain
    ScaleDown,
}

impl Property for ObjectFit {
    fn static_default() -> &'static Self {
        &Self::Contain
    }
}

impl Default for ObjectFit {
    fn default() -> Self {
        *Self::static_default()
    }
}

// TODO - Need to write tests for this, in a way that's relatively easy to visualize.

impl ObjectFit {
    /// Helper function to be called in [`Widget::property_changed`](crate::core::Widget::property_changed).
    pub fn prop_changed(ctx: &mut UpdateCtx<'_>, property_type: TypeId) {
        if property_type != TypeId::of::<Self>() {
            return;
        }
        ctx.request_layout();
    }

    /// Calculates an origin and scale for an image with a given `ObjectFit`.
    ///
    /// This takes some properties of a widget and an object fit and returns an affine matrix
    /// used to position and scale the image in the widget.
    pub fn affine_to_fill(self, parent: Size, fit_box: Size) -> Affine {
        if fit_box.width == 0. || fit_box.height == 0. {
            return Affine::IDENTITY;
        }

        let raw_scalex = parent.width / fit_box.width;
        let raw_scaley = parent.height / fit_box.height;

        let (scalex, scaley) = match self {
            Self::Contain => {
                let scale = raw_scalex.min(raw_scaley);
                (scale, scale)
            }
            Self::Cover => {
                let scale = raw_scalex.max(raw_scaley);
                (scale, scale)
            }
            Self::Fill => (raw_scalex, raw_scaley),
            Self::FitHeight => (raw_scaley, raw_scaley),
            Self::FitWidth => (raw_scalex, raw_scalex),
            Self::ScaleDown => {
                let scale = raw_scalex.min(raw_scaley).min(1.0);
                (scale, scale)
            }
            Self::None => (1.0, 1.0),
        };

        let origin_x = (parent.width - (fit_box.width * scalex)) / 2.0;
        let origin_y = (parent.height - (fit_box.height * scaley)) / 2.0;

        Affine::new([scalex, 0., 0., scaley, origin_x, origin_y])
    }
}
