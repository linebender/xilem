// Copyright 2025 the Xilem Authors
// SPDX-License-Identifier: Apache-2.0

use crate::{layout::LenReq, util::Sanitize};

/// Widget length definition.
///
/// This is an intermediate representation of widget length,
/// used after resolving [`Dim`] but before potentially measuring the widget.
///
/// This is how a parent specifies [`Dim::Auto`] behavior for its children.
///
/// All the values must be finite, non-negative, and in device pixels.
///
/// [`Dim`]: crate::layout::Dim
/// [`Dim::Auto`]: crate::layout::Dim::Auto
#[derive(Copy, Clone, Debug, PartialEq)]
pub enum LenDef {
    /// Specific fixed length.
    ///
    /// The value must be finite, non-negative, and in device pixels.
    Fixed(f64),
    /// Intrinsic minimum length.
    ///
    /// This will result in a [`measure`] invocation, which can be slow.
    ///
    /// [`measure`]: crate::core::Widget::measure
    MinContent,
    /// Intrinsic preferred length.
    ///
    /// This will result in a [`measure`] invocation, which can be slow.
    ///
    /// [`measure`]: crate::core::Widget::measure
    MaxContent,
    /// The content should fit in the specified available space.
    ///
    /// The value must be finite, non-negative, and in device pixels.
    ///
    /// This will result in a [`measure`] invocation, which can be slow.
    ///
    /// [`measure`]: crate::core::Widget::measure
    FitContent(f64),
}

impl From<LenReq> for LenDef {
    fn from(len_req: LenReq) -> Self {
        match len_req {
            LenReq::MinContent => Self::MinContent,
            LenReq::MaxContent => Self::MaxContent,
            LenReq::FitContent(space) => Self::FitContent(space),
        }
    }
}

impl LenDef {
    /// Returns the specific fixed length if it is present.
    ///
    /// The length will be in device pixels.
    ///
    /// Whether the length can be non-finite or negative depends on whether
    /// this [`LenDef`] has been [sanitized].
    ///
    /// [sanitized]: LenDef::sanitize
    pub fn fixed(&self) -> Option<f64> {
        match self {
            Self::Fixed(val) => Some(*val),
            _ => None,
        }
    }

    /// Returns [`LenDef`] with `delta` subtracted from it.
    ///
    /// [`Fixed`] and [`FitContent`] will have their value reduced by `delta`, but clamped to zero.
    /// [`MinContent`] and [`MaxContent`] are returned as-is.
    ///
    /// The provided `delta` must be in device pixels.
    ///
    /// [`Fixed`]: Self::Fixed
    /// [`FitContent`]: Self::FitContent
    /// [`MinContent`]: Self::MinContent
    /// [`MaxContent`]: Self::MaxContent
    pub fn reduce(self, delta: f64) -> Self {
        match self {
            Self::Fixed(val) => Self::Fixed((val - delta).max(0.)),
            Self::MinContent | Self::MaxContent => self,
            Self::FitContent(space) => Self::FitContent((space - delta).max(0.)),
        }
    }
}

impl Sanitize for LenDef {
    /// Returns a valid instance of [`LenDef`].
    ///
    /// It will return [`MaxContent`] if the [`Fixed`] or [`FitContent`]
    /// values are non-finite or negative.
    ///
    /// This method is called by Masonry during the layout pass,
    /// when a widget's length is being resolved.
    ///
    /// # Panics
    ///
    /// Panics if the [`Fixed`] or [`FitContent`] values are non-finite or negative
    /// and debug assertions are enabled.
    ///
    /// [`Fixed`]: Self::Fixed
    /// [`FitContent`]: Self::FitContent
    /// [`MaxContent`]: Self::MaxContent
    #[track_caller]
    fn sanitize(self, name: &str) -> Self {
        match self {
            Self::Fixed(val) => {
                if val.is_finite() && val >= 0. {
                    self
                } else {
                    debug_panic!(
                        "{name} `Fixed` value must be finite and non-negative. Received: {val}"
                    );
                    Self::MaxContent
                }
            }
            Self::MinContent | Self::MaxContent => self,
            Self::FitContent(space) => {
                if space.is_finite() && space >= 0. {
                    self
                } else {
                    debug_panic!(
                        "{name} `FitContent` value must be finite and non-negative. Received: {space}"
                    );
                    Self::MaxContent
                }
            }
        }
    }
}
