// Copyright 2025 the Xilem Authors
// SPDX-License-Identifier: Apache-2.0

use crate::util::Sanitize;

/// Widget length measurement algorithm request.
///
/// It is up to the widget itself to define how it responds to these requests.
#[derive(Copy, Clone, Debug, PartialEq)]
pub enum LenReq {
    /// The widget should measure its minimum intrinsic length.
    MinContent,
    /// The widget should measure its maximum intrinsic length.
    MaxContent,
    /// The widget should attempt to fit into the specified available space.
    ///
    /// The space value must be finite, non-negative, and in device pixels.
    FitContent(f64),
}

impl LenReq {
    /// Returns [`LenReq`] with `delta` subtracted from it.
    ///
    /// [`FitContent`] will have its value reduced by `delta`, but clamped to zero.
    /// [`MinContent`] and [`MaxContent`] are returned as-is.
    ///
    /// The provided `delta` must be in device pixels.
    ///
    /// [`FitContent`]: Self::FitContent
    /// [`MinContent`]: Self::MinContent
    /// [`MaxContent`]: Self::MaxContent
    pub fn reduce(self, delta: f64) -> Self {
        match self {
            Self::MinContent | Self::MaxContent => self,
            Self::FitContent(space) => Self::FitContent((space - delta).max(0.)),
        }
    }
}

impl Sanitize for LenReq {
    /// Returns a valid instance of [`LenReq`].
    ///
    /// It will return [`MaxContent`] if the [`FitContent`] value is non-finite or negative.
    ///
    /// # Panics
    ///
    /// Panics if [`FitContent`] value is non-finite or negative and debug assertions are enabled.
    ///
    /// [`FitContent`]: Self::FitContent
    /// [`MaxContent`]: Self::MaxContent
    #[track_caller]
    fn sanitize(self, name: &str) -> Self {
        match self {
            Self::MinContent | Self::MaxContent => self,
            Self::FitContent(space) => {
                if space.is_finite() && space >= 0. {
                    self
                } else {
                    debug_panic!(
                        "{name} `space` must be finite and non-negative. Received: {space}"
                    );
                    Self::MaxContent
                }
            }
        }
    }
}
