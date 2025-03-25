// Copyright 2025 the Xilem Authors
// SPDX-License-Identifier: Apache-2.0

use crate::kurbo::{Affine, Size};

// These are based on https://developer.mozilla.org/en-US/docs/Web/CSS/object-fit
/// Strategies for inscribing a rectangle inside another rectangle.
#[derive(Clone, Copy, Default, PartialEq)]
pub enum ObjectFit {
    /// As large as possible without changing aspect ratio of image and all of image shown
    #[default]
    Contain,
    /// As large as possible with no dead space so that some of the image may be clipped
    Cover,
    /// Fill the widget with no dead space, aspect ratio of widget is used
    Fill,
    /// Fill the height with the images aspect ratio, some of the image may be clipped
    FitHeight,
    /// Fill the width with the images aspect ratio, some of the image may be clipped
    FitWidth,
    /// Do not scale
    None,
    /// Scale down to fit but do not scale up
    ScaleDown,
}

// TODO - Need to write tests for this, in a way that's relatively easy to visualize.

impl ObjectFit {
    /// Calculate an origin and scale for an image with a given `ObjectFit`.
    ///
    /// This takes some properties of a widget and an object fit and returns an affine matrix
    /// used to position and scale the image in the widget.
    pub fn affine_to_fill(self, parent: Size, fit_box: Size) -> Affine {
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
