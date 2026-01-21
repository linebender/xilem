// Copyright 2025 the Xilem Authors
// SPDX-License-Identifier: Apache-2.0

use std::any::TypeId;

use crate::core::Property;
use crate::peniko::color::{AlphaColor, Srgb};

/// The color of a [`TextInput`]'s placeholder text.
///
/// [`TextInput`]: crate::widgets::TextInput
#[expect(missing_docs, reason = "field names are self-descriptive")]
#[derive(Clone, Copy, Debug, PartialEq)]
pub struct PlaceholderColor {
    pub color: AlphaColor<Srgb>,
}

impl Property for PlaceholderColor {
    fn static_default() -> &'static Self {
        static DEFAULT: PlaceholderColor = PlaceholderColor {
            color: AlphaColor::BLACK,
        };
        &DEFAULT
    }

    #[inline(always)]
    fn matches(property_type: TypeId) -> bool {
        property_type == TypeId::of::<Self>()
    }
}

impl PlaceholderColor {
    /// Creates new `PlaceholderColor` with given value.
    pub const fn new(color: AlphaColor<Srgb>) -> Self {
        Self { color }
    }
}

// ---

impl Default for PlaceholderColor {
    fn default() -> Self {
        *Self::static_default()
    }
}
