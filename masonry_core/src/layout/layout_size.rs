// Copyright 2025 the Xilem Authors
// SPDX-License-Identifier: Apache-2.0

use vello::kurbo::{Axis, Size};

use crate::util::Sanitize;

/// Layout width and height.
///
/// A length may be missing if it has not been computed yet, i.e. it depends on the child.
///
/// The lengths, if present, are always finite, non-negative, and in device pixels.
#[derive(Copy, Clone, Debug, Default, PartialEq)]
pub struct LayoutSize {
    width: Option<f64>,
    height: Option<f64>,
}

impl From<Size> for LayoutSize {
    fn from(size: Size) -> Self {
        let width = Some(size.width).sanitize("LayoutSize width");
        let height = Some(size.height).sanitize("LayoutSize height");
        Self { width, height }
    }
}

impl LayoutSize {
    /// No size info is available.
    pub const NONE: Self = Self {
        width: None,
        height: None,
    };

    /// Creates a new [`LayoutSize`] with the given lengths.
    ///
    /// The lengths must be finite, non-negative, and in device pixels.
    /// Invalid lengths will result in `None`.
    ///
    /// # Panics
    ///
    /// Panics if `width` or `height` are non-finite or negative
    /// and debug assertions are enabled.
    pub fn new(width: f64, height: f64) -> Self {
        let width = Some(width).sanitize("LayoutSize width");
        let height = Some(height).sanitize("LayoutSize height");
        Self { width, height }
    }

    /// Creates a new [`LayoutSize`] with only the given `axis` set to `length`.
    ///
    /// The length must be finite, non-negative, and in device pixels.
    /// An invalid length will result in `None`.
    ///
    /// # Panics
    ///
    /// Panics if `length` is non-finite or negative and debug assertions are enabled.
    pub fn one(axis: Axis, length: f64) -> Self {
        let length = Some(length).sanitize("LayoutSize length");
        match axis {
            Axis::Horizontal => Self {
                width: length,
                height: None,
            },
            Axis::Vertical => Self {
                width: None,
                height: length,
            },
        }
    }

    /// Creates a new [`LayoutSize`] with only the given `axis` set to `length`.
    ///
    /// The length, if present, must be finite, non-negative, and in device pixels.
    /// An invalid length will result in `None`.
    ///
    /// # Panics
    ///
    /// Panics if `length` is present but non-finite or negative
    /// and debug assertions are enabled.
    pub fn maybe(axis: Axis, length: Option<f64>) -> Self {
        let Some(length) = length else {
            return Self {
                width: None,
                height: None,
            };
        };
        Self::one(axis, length)
    }

    /// Returns the [`Length`] of the provided `axis`.
    ///
    /// The returned value will be finite, non-negative, and in device pixels.
    ///
    /// [`Length`]: crate::layout::Length
    pub const fn length(&self, axis: Axis) -> Option<f64> {
        match axis {
            Axis::Horizontal => self.width,
            Axis::Vertical => self.height,
        }
    }
}
