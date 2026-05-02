// Copyright 2025 the Xilem Authors
// SPDX-License-Identifier: Apache-2.0

use crate::layout::{LenReq, Length};

/// Widget border-box length definition.
///
/// This is an intermediate representation of widget border-box length,
/// used after resolving [`Dim`] but before potentially measuring the widget.
///
/// This is how a parent specifies [`Dim::Auto`] behavior for its children.
///
/// [`Dim`]: crate::layout::Dim
/// [`Dim::Auto`]: crate::layout::Dim::Auto
#[derive(Copy, Clone, Debug, PartialEq)]
pub enum LenDef {
    /// Specific fixed border-box [`Length`].
    Fixed(Length),
    /// Minimum preferred border-box length.
    ///
    /// This will result in a [`measure`] invocation, which can be slow.
    ///
    /// [`measure`]: crate::core::Widget::measure
    MinContent,
    /// Maximum preferred border-box length.
    ///
    /// This will result in a [`measure`] invocation, which can be slow.
    ///
    /// [`measure`]: crate::core::Widget::measure
    MaxContent,
    /// The border-box should fit in the specified available space.
    ///
    /// This will result in a [`measure`] invocation, which can be slow.
    ///
    /// [`measure`]: crate::core::Widget::measure
    FitContent(Length),
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
    /// Returns the specific fixed border-box length if it is present.
    pub fn fixed(&self) -> Option<Length> {
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
    /// [`Fixed`]: Self::Fixed
    /// [`FitContent`]: Self::FitContent
    /// [`MinContent`]: Self::MinContent
    /// [`MaxContent`]: Self::MaxContent
    pub fn reduce(self, delta: Length) -> Self {
        match self {
            Self::Fixed(val) => Self::Fixed(val.saturating_sub(delta)),
            Self::MinContent | Self::MaxContent => self,
            Self::FitContent(space) => Self::FitContent(space.saturating_sub(delta)),
        }
    }
}
