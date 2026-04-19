// Copyright 2025 the Xilem Authors
// SPDX-License-Identifier: Apache-2.0

use crate::core::{Property, UsesProperty, Widget};
use crate::peniko::BrushRef;
use crate::peniko::color::{AlphaColor, Srgb};

// Every widget has a border color.
impl<W: Widget> UsesProperty<BorderColor> for W {}

/// The color of a widget's border.
#[expect(missing_docs, reason = "field names are self-descriptive")]
#[derive(Clone, Copy, Debug, PartialEq)]
pub struct BorderColor {
    pub color: AlphaColor<Srgb>,
}

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
