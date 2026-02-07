// Copyright 2025 the Xilem Authors
// SPDX-License-Identifier: Apache-2.0

use crate::core::{HasProperty, Property, Widget};
use crate::kurbo::Rect;
use crate::peniko::color::{AlphaColor, Srgb};
use crate::properties::types::Gradient;

// TODO - Replace "Background" with "BackgroundColor" and move the gradient case
// to BackgroundImage to match CSS spec.

// Every widget has a background.
impl<W: Widget> HasProperty<DisabledBackground> for W {}
impl<W: Widget> HasProperty<ActiveBackground> for W {}
impl<W: Widget> HasProperty<Background> for W {}

/// The background color/gradient of a widget.
#[expect(missing_docs, reason = "field names are self-descriptive")]
#[derive(Clone, Debug, PartialEq)]
pub enum Background {
    Color(AlphaColor<Srgb>),
    Gradient(Gradient),
}

/// The background color/gradient a widget takes when the user is clicking or otherwise using it.
#[derive(Clone, Debug, PartialEq)]
pub struct ActiveBackground(pub Background);

/// The background color/gradient a widget takes when disabled.
#[derive(Clone, Debug, PartialEq)]
pub struct DisabledBackground(pub Background);

// ---

impl Property for Background {
    fn static_default() -> &'static Self {
        // This matches the CSS default.
        static DEFAULT: Background = Background::Color(AlphaColor::TRANSPARENT);
        &DEFAULT
    }
}

impl Default for Background {
    fn default() -> Self {
        Self::static_default().clone()
    }
}

impl Background {
    /// Returns a brush that can be used for a `fill` operation.
    ///
    /// If `Self` is a `Color`, this returns a solid color brush.
    /// If `Self` is a `Gradient` this returns a gradient filling the given rect according to
    /// CSS spec.
    ///
    /// (See [`Gradient::get_peniko_gradient_for_rect`])
    pub fn get_peniko_brush_for_rect(&self, rect: Rect) -> crate::peniko::Brush {
        match self {
            Self::Color(color) => (*color).into(),
            Self::Gradient(gradient) => gradient.get_peniko_gradient_for_rect(rect).into(),
        }
    }

    /// Returns `false` if the background can be safely treated as non-existent.
    ///
    /// May have false positives.
    pub const fn is_visible(&self) -> bool {
        match self {
            Self::Color(color) => {
                let alpha = color.components[3];
                alpha != 0.0
            }
            Self::Gradient(_) => true,
        }
    }
}

// ---

impl Property for ActiveBackground {
    fn static_default() -> &'static Self {
        // This matches the CSS default.
        const DEFAULT: ActiveBackground =
            ActiveBackground(Background::Color(AlphaColor::TRANSPARENT));
        &DEFAULT
    }
}

impl Default for ActiveBackground {
    fn default() -> Self {
        Self::static_default().clone()
    }
}

// ---

impl Property for DisabledBackground {
    fn static_default() -> &'static Self {
        // This matches the CSS default.
        const DEFAULT: DisabledBackground =
            DisabledBackground(Background::Color(AlphaColor::TRANSPARENT));
        &DEFAULT
    }
}

impl Default for DisabledBackground {
    fn default() -> Self {
        Self::static_default().clone()
    }
}
