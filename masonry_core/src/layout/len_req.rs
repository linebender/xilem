// Copyright 2025 the Xilem Authors
// SPDX-License-Identifier: Apache-2.0

use crate::layout::Length;

/// Widget length measurement algorithm request.
///
/// It is up to the widget itself to define how it responds to these requests.
#[derive(Copy, Clone, Debug, PartialEq)]
pub enum LenReq {
    /// The widget should measure its minimum preferred length.
    MinContent,
    /// The widget should measure its maximum preferred length.
    MaxContent,
    /// The widget should attempt to fit into the specified available space.
    FitContent(Length),
}

impl LenReq {
    /// Returns [`LenReq`] with `delta` subtracted from it.
    ///
    /// [`FitContent`] will have its value reduced by `delta`, but clamped to zero.
    /// [`MinContent`] and [`MaxContent`] are returned as-is.
    ///
    /// [`FitContent`]: Self::FitContent
    /// [`MinContent`]: Self::MinContent
    /// [`MaxContent`]: Self::MaxContent
    pub fn reduce(self, delta: Length) -> Self {
        match self {
            Self::MinContent | Self::MaxContent => self,
            Self::FitContent(space) => Self::FitContent(space.saturating_sub(delta)),
        }
    }
}
