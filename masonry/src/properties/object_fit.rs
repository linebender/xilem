// Copyright 2025 the Xilem Authors
// SPDX-License-Identifier: Apache-2.0

use std::any::TypeId;

use crate::core::{Property, UpdateCtx};
use crate::kurbo::{Affine, Size};
use crate::util::Sanitize;

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
    /// The content is stretched to fully fill the container.
    ///
    /// If the content's aspect ratio does not match the aspect ratio of its container,
    /// then the content will be stretched to fit exactly, changing its aspect ratio.
    Stretch,
}

impl Property for ObjectFit {
    fn static_default() -> &'static Self {
        &Self::Contain
    }

    #[inline(always)]
    fn matches(property_type: TypeId) -> bool {
        property_type == TypeId::of::<Self>()
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

    /// Calculates an [`Affine`] transform to fit `content` inside `container`.
    ///
    /// See [`ObjectFit`] variant documentation for fitting details.
    ///
    /// # Panics
    ///
    /// Panics if either `content` or `container` is non-finite or negative
    /// and debug assertions are enabled.
    pub fn affine(self, container: Size, content: Size) -> Affine {
        // Guard against invalid input
        let container = Size::new(
            container.width.sanitize("container width"),
            container.height.sanitize("container height"),
        );
        let content = Size::new(
            content.width.sanitize("content width"),
            content.height.sanitize("content height"),
        );
        // Guard against division by zero
        if content.width == 0. || content.height == 0. {
            return Affine::IDENTITY;
        }

        let raw_scalex = container.width / content.width;
        let raw_scaley = container.height / content.height;

        let (scalex, scaley) = match self {
            Self::Contain => {
                let scale = raw_scalex.min(raw_scaley);
                (scale, scale)
            }
            Self::Cover => {
                let scale = raw_scalex.max(raw_scaley);
                (scale, scale)
            }
            Self::FitHeight => (raw_scaley, raw_scaley),
            Self::FitWidth => (raw_scalex, raw_scalex),
            Self::None => (1.0, 1.0),
            Self::ScaleDown => {
                let scale = raw_scalex.min(raw_scaley).min(1.0);
                (scale, scale)
            }
            Self::Stretch => (raw_scalex, raw_scaley),
        };

        let origin_x = (container.width - (content.width * scalex)) * 0.5;
        let origin_y = (container.height - (content.height * scaley)) * 0.5;

        Affine::new([scalex, 0., 0., scaley, origin_x, origin_y])
    }
}
