// Copyright 2025 the Xilem Authors
// SPDX-License-Identifier: Apache-2.0

use crate::core::{HasProperty, Property, Widget};
use crate::peniko::BrushRef;
use crate::peniko::color::{AlphaColor, Srgb};

// Every widget has a border color.
impl<W: Widget> HasProperty<FocusedBorderColor> for W {}
impl<W: Widget> HasProperty<HoveredBorderColor> for W {}
impl<W: Widget> HasProperty<BorderColor> for W {}

/// The color of a widget's border.
#[expect(missing_docs, reason = "field names are self-descriptive")]
#[derive(Clone, Copy, Debug, PartialEq)]
pub struct BorderColor {
    pub color: AlphaColor<Srgb>,
}

/// The color of a widget's border when hovered by a pointer.
#[derive(Clone, Copy, Debug, PartialEq)]
pub struct HoveredBorderColor(pub BorderColor);

/// The color of a widget's border when focused.
#[derive(Clone, Copy, Debug, PartialEq)]
pub struct FocusedBorderColor(pub BorderColor);

// ---

// TODO - The default border color in CSS is `currentcolor`,
// the color text is displayed in.
// Do we want to implement that?

impl Property for BorderColor {
    fn static_default() -> &'static Self {
        static DEFAULT: BorderColor = BorderColor {
            color: AlphaColor::TRANSPARENT,
        };
        &DEFAULT
    }
}

impl Default for BorderColor {
    fn default() -> Self {
        *Self::static_default()
    }
}

impl<'a> From<&'a BorderColor> for BrushRef<'a> {
    fn from(color: &'a BorderColor) -> Self {
        Self::Solid(color.color)
    }
}

impl BorderColor {
    /// Creates new `BorderColor` with given value.
    pub const fn new(color: AlphaColor<Srgb>) -> Self {
        Self { color }
    }

    /// Returns `false` if the color can be safely treated as non-existent.
    ///
    /// May have false positives.
    pub const fn is_visible(&self) -> bool {
        let alpha = self.color.components[3];
        alpha != 0.0
    }
}

// ---

impl Default for HoveredBorderColor {
    fn default() -> Self {
        *Self::static_default()
    }
}

impl Property for HoveredBorderColor {
    fn static_default() -> &'static Self {
        static DEFAULT: HoveredBorderColor = HoveredBorderColor(BorderColor {
            color: AlphaColor::TRANSPARENT,
        });
        &DEFAULT
    }
}

// ---

impl Default for FocusedBorderColor {
    fn default() -> Self {
        *Self::static_default()
    }
}

impl Property for FocusedBorderColor {
    fn static_default() -> &'static Self {
        static DEFAULT: FocusedBorderColor = FocusedBorderColor(BorderColor {
            color: AlphaColor::TRANSPARENT,
        });
        &DEFAULT
    }
}
