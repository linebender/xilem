// Copyright 2018 the Xilem Authors and the Druid Authors
// SPDX-License-Identifier: Apache-2.0

//! Font attributes

use parley::{
    fontique::{Style, Weight},
    style::FontFamily,
};

/// A collection of attributes that describe a font.
///
/// This is provided as a convenience; library consumers may wish to have
/// a single type that represents a specific font face at a specific size.
#[derive(Debug, Clone, PartialEq)]
pub struct FontDescriptor {
    /// The font's [`FontFamily`](struct.FontFamily.html).
    pub family: FontFamily<'static>,
    /// The font's size.
    pub size: f32,
    /// The font's [`FontWeight`](struct.FontWeight.html).
    pub weight: Weight,
    /// The font's [`FontStyle`](struct.FontStyle.html).
    pub style: Style,
}

impl FontDescriptor {
    /// Create a new descriptor with the provided [`FontFamily`].
    ///
    /// [`FontFamily`]: struct.FontFamily.html
    pub const fn new(family: FontFamily<'static>) -> Self {
        FontDescriptor {
            family,
            size: 12.,
            weight: Weight::NORMAL,
            style: Style::Normal,
        }
    }

    /// Buider-style method to set the descriptor's font size.
    pub const fn with_size(mut self, size: f32) -> Self {
        self.size = size;
        self
    }

    /// Buider-style method to set the descriptor's [`FontWeight`].
    ///
    /// [`FontWeight`]: struct.FontWeight.html
    pub const fn with_weight(mut self, weight: Weight) -> Self {
        self.weight = weight;
        self
    }

    /// Buider-style method to set the descriptor's [`FontStyle`].
    ///
    /// [`FontStyle`]: enum.FontStyle.html
    pub const fn with_style(mut self, style: Style) -> Self {
        self.style = style;
        self
    }
}

impl Default for FontDescriptor {
    fn default() -> Self {
        FontDescriptor {
            family: FontFamily::Generic(parley::style::GenericFamily::SystemUi),
            weight: Default::default(),
            style: Default::default(),
            size: 12.,
        }
    }
}
