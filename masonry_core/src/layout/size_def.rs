// Copyright 2025 the Xilem Authors
// SPDX-License-Identifier: Apache-2.0

use vello::kurbo::{Axis, Size};

use crate::layout::LenDef;
use crate::util::Sanitize;

/// Widget border-box size definition.
///
/// This is how a parent specifies [`Dim::Auto`] behavior for its children.
///
/// The inner [`LenDef`] values will already be [sanitized] and are safe to read.
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
    ///
    /// All the [`LenDef`] values must be finite, non-negative, and in device pixels.
    /// Invalid [`LenDef`] values will fall back to [`LenDef::MaxContent`].
    ///
    /// # Panics
    ///
    /// Panics if `width` or `height` contain non-finite or negative values
    /// and debug assertions are enabled.
    pub fn new(width: LenDef, height: LenDef) -> Self {
        let width = width.sanitize("SizeDef width");
        let height = height.sanitize("SizeDef height");
        Self { width, height }
    }

    /// Creates a new [`SizeDef`] with both axes set to [`LenDef::FitContent`].
    ///
    /// See [`LenDef::FitContent`] for details.
    ///
    /// The `size` must be finite, non-negative, and in device pixels.
    /// An invalid `size` dimension value will fall back to [`LenDef::MaxContent`].
    ///
    /// # Panics
    ///
    /// Panics if `size` contains non-finite or negative values and debug assertions are enabled.
    pub fn fit(size: Size) -> Self {
        Self::new(
            LenDef::FitContent(size.width),
            LenDef::FitContent(size.height),
        )
    }

    /// Creates a new [`SizeDef`] with both axes set to [`LenDef::Fixed`].
    ///
    /// See [`LenDef::Fixed`] for details.
    ///
    /// The `size` must be finite, non-negative, and in device pixels.
    /// An invalid `size` dimension value will fall back to [`LenDef::MaxContent`].
    ///
    /// # Panics
    ///
    /// Panics if `size` contains non-finite or negative values and debug assertions are enabled.
    pub fn fixed(size: Size) -> Self {
        Self::new(LenDef::Fixed(size.width), LenDef::Fixed(size.height))
    }

    /// Creates a new [`SizeDef`] with `axis` set to [`LenDef`].
    ///
    /// The other axis will be [`LenDef::MaxContent`].
    ///
    /// [`LenDef`] values must be finite, non-negative, and in device pixels.
    /// Invalid [`LenDef`] values will fall back to [`LenDef::MaxContent`].
    ///
    /// # Panics
    ///
    /// Panics if `len_def` contains non-finite or negative values and debug assertions are enabled.
    pub fn one(axis: Axis, len_def: LenDef) -> Self {
        let len_def = len_def.sanitize("SizeDef::one len_def");
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
    ///
    /// [`LenDef`] values must be finite, non-negative, and in device pixels.
    /// Invalid [`LenDef`] values will fall back to [`LenDef::MaxContent`].
    ///
    /// # Panics
    ///
    /// Panics if `len_def` contains non-finite or negative values and debug assertions are enabled.
    pub fn with(self, axis: Axis, len_def: LenDef) -> Self {
        match axis {
            Axis::Horizontal => self.with_width(len_def),
            Axis::Vertical => self.with_height(len_def),
        }
    }

    /// Returns the [`SizeDef`] with the width set to [`LenDef`].
    ///
    /// [`LenDef`] values must be finite, non-negative, and in device pixels.
    /// Invalid [`LenDef`] values will fall back to [`LenDef::MaxContent`].
    ///
    /// # Panics
    ///
    /// Panics if `len_def` contains non-finite or negative values and debug assertions are enabled.
    pub fn with_width(mut self, len_def: LenDef) -> Self {
        self.width = len_def.sanitize("SizeDef width");
        self
    }

    /// Returns the [`SizeDef`] with the height set to [`LenDef`].
    ///
    /// [`LenDef`] values must be finite, non-negative, and in device pixels.
    /// Invalid [`LenDef`] values will fall back to [`LenDef::MaxContent`].
    ///
    /// # Panics
    ///
    /// Panics if `len_def` contains non-finite or negative values and debug assertions are enabled.
    pub fn with_height(mut self, len_def: LenDef) -> Self {
        self.height = len_def.sanitize("SizeDef height");
        self
    }

    /// Returns the [`LenDef`] of the given `axis`.
    ///
    /// The result will already have been [sanitized].
    ///
    /// [sanitized]: Sanitize
    pub const fn dim(&self, axis: Axis) -> LenDef {
        match axis {
            Axis::Horizontal => self.width,
            Axis::Vertical => self.height,
        }
    }
}
