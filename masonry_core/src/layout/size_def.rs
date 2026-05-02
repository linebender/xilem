// Copyright 2025 the Xilem Authors
// SPDX-License-Identifier: Apache-2.0

use kurbo::{Axis, Size};

use crate::layout::{LenDef, Length};
use crate::util::Sanitize;

/// Widget border-box size definition.
///
/// This is how a parent specifies [`Dim::Auto`] behavior for its children.
///
/// [`Dim::Auto`]: crate::layout::Dim::Auto
/// [sanitized]: Sanitize
#[derive(Copy, Clone, Debug, PartialEq)]
pub struct SizeDef {
    width: LenDef,
    height: LenDef,
}

impl SizeDef {
    /// Minimum preferred border-box size.
    ///
    /// See [`LenDef::MinContent`] for details.
    pub const MIN: Self = Self {
        width: LenDef::MinContent,
        height: LenDef::MinContent,
    };

    /// Maximum preferred border-box size.
    ///
    /// See [`LenDef::MaxContent`] for details.
    pub const MAX: Self = Self {
        width: LenDef::MaxContent,
        height: LenDef::MaxContent,
    };

    /// Creates a new [`SizeDef`] with the given `width` and `height`.
    pub fn new(width: LenDef, height: LenDef) -> Self {
        Self { width, height }
    }

    /// Creates a new [`SizeDef`] with both axes set to [`LenDef::FitContent`].
    ///
    /// See [`LenDef::FitContent`] for details.
    ///
    /// The `size` must be finite, non-negative, and in logical pixels.
    /// An invalid `size` dimension value will fall back to zero.
    ///
    /// # Panics
    ///
    /// Panics if `size` contains non-finite or negative values and debug assertions are enabled.
    pub fn fit(size: Size) -> Self {
        let width = size.width.sanitize("SizeDef::fit width");
        let height = size.height.sanitize("SizeDef::fit height");
        Self::new(
            LenDef::FitContent(Length::px(width)),
            LenDef::FitContent(Length::px(height)),
        )
    }

    /// Creates a new [`SizeDef`] with both axes set to [`LenDef::Fixed`].
    ///
    /// See [`LenDef::Fixed`] for details.
    ///
    /// The `size` must be finite, non-negative, and in logical pixels.
    /// An invalid `size` dimension value will fall back to zero.
    ///
    /// # Panics
    ///
    /// Panics if `size` contains non-finite or negative values and debug assertions are enabled.
    pub fn fixed(size: Size) -> Self {
        let width = size.width.sanitize("SizeDef::fixed width");
        let height = size.height.sanitize("SizeDef::fixed height");
        Self::new(
            LenDef::Fixed(Length::px(width)),
            LenDef::Fixed(Length::px(height)),
        )
    }

    /// Creates a new [`SizeDef`] with `axis` set to [`LenDef`].
    ///
    /// The other axis will be [`LenDef::MaxContent`].
    pub fn one(axis: Axis, len_def: LenDef) -> Self {
        match axis {
            Axis::Horizontal => Self {
                width: len_def,
                height: LenDef::MaxContent,
            },
            Axis::Vertical => Self {
                width: LenDef::MaxContent,
                height: len_def,
            },
        }
    }

    /// Returns the [`SizeDef`] with `axis` set to [`LenDef`].
    pub fn with(self, axis: Axis, len_def: LenDef) -> Self {
        match axis {
            Axis::Horizontal => self.with_width(len_def),
            Axis::Vertical => self.with_height(len_def),
        }
    }

    /// Returns the [`SizeDef`] with the width set to [`LenDef`].
    pub fn with_width(mut self, len_def: LenDef) -> Self {
        self.width = len_def;
        self
    }

    /// Returns the [`SizeDef`] with the height set to [`LenDef`].
    pub fn with_height(mut self, len_def: LenDef) -> Self {
        self.height = len_def;
        self
    }

    /// Returns the [`LenDef`] of the given `axis`.
    pub const fn dim(&self, axis: Axis) -> LenDef {
        match axis {
            Axis::Horizontal => self.width,
            Axis::Vertical => self.height,
        }
    }
}
