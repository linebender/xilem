// Copyright 2026 the Xilem Authors
// SPDX-License-Identifier: Apache-2.0

use crate::core::Property;
use crate::peniko::color::{AlphaColor, Srgb};

/// The visual structure of the [`StepInput`] widget.
///
/// [`StepInput`]: crate::widgets::StepInput
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum StepInputStyle {
    /// Basic style with simple plus and minus buttons.
    Basic,
    /// The buttons are outward facing arrows.
    ///
    /// During slide mode the arrows move and have a trail.
    /// The intensity and size of the trail increases with slide speed.
    Flow,
}

impl Property for StepInputStyle {
    fn static_default() -> &'static Self {
        &Self::Flow
    }
}

impl Default for StepInputStyle {
    fn default() -> Self {
        *Self::static_default()
    }
}

/// The color of backwards stepping controls when active/hovered.
#[derive(Clone, Copy, Debug, PartialEq)]
pub struct BackwardColor(pub AlphaColor<Srgb>);

impl Property for BackwardColor {
    fn static_default() -> &'static Self {
        /// Purple
        static DEFAULT: BackwardColor = BackwardColor(AlphaColor::from_rgb8(0xa7, 0x90, 0xd2));
        &DEFAULT
    }
}

impl Default for BackwardColor {
    fn default() -> Self {
        *Self::static_default()
    }
}

impl BackwardColor {
    /// Creates a new `BackwardColor` with the given `color`.
    pub const fn new(color: AlphaColor<Srgb>) -> Self {
        Self(color)
    }
}

/// The color of forwards stepping controls when active/hovered.
#[derive(Clone, Copy, Debug, PartialEq)]
pub struct ForwardColor(pub AlphaColor<Srgb>);

impl Property for ForwardColor {
    fn static_default() -> &'static Self {
        /// Blue
        static DEFAULT: ForwardColor = ForwardColor(AlphaColor::from_rgb8(0x2a, 0xd5, 0xe4));
        &DEFAULT
    }
}

impl Default for ForwardColor {
    fn default() -> Self {
        *Self::static_default()
    }
}

impl ForwardColor {
    /// Creates a new `ForwardColor` with the given `color`.
    pub const fn new(color: AlphaColor<Srgb>) -> Self {
        Self(color)
    }
}

/// The color of arrow tips in the [`Flow`] style.
///
/// [`Flow`]: StepInputStyle::Flow
#[derive(Clone, Copy, Debug, PartialEq)]
pub struct HeatColor(pub AlphaColor<Srgb>);

impl Property for HeatColor {
    fn static_default() -> &'static Self {
        static DEFAULT: HeatColor = HeatColor(AlphaColor::WHITE);
        &DEFAULT
    }
}

impl Default for HeatColor {
    fn default() -> Self {
        *Self::static_default()
    }
}

impl HeatColor {
    /// Creates a new `ForwardColor` with the given `color`.
    pub const fn new(color: AlphaColor<Srgb>) -> Self {
        Self(color)
    }
}
