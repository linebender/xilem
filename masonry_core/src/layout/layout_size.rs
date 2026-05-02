// Copyright 2025 the Xilem Authors
// SPDX-License-Identifier: Apache-2.0

use kurbo::{Axis, Size};

use crate::layout::Length;

/// Layout width and height.
///
/// A length may be missing if it has not been computed yet, i.e. it depends on the child.
#[derive(Copy, Clone, Debug, Default, PartialEq)]
pub struct LayoutSize {
    width: Option<Length>,
    height: Option<Length>,
}

impl From<Size> for LayoutSize {
    #[track_caller]
    fn from(size: Size) -> Self {
        Self {
            width: Some(Length::px(size.width)),
            height: Some(Length::px(size.height)),
        }
    }
}

impl LayoutSize {
    /// No size info is available.
    pub const NONE: Self = Self {
        width: None,
        height: None,
    };

    /// Creates a new [`LayoutSize`] with the given lengths.
    pub fn new(width: Length, height: Length) -> Self {
        Self {
            width: Some(width),
            height: Some(height),
        }
    }

    /// Creates a new [`LayoutSize`] with only the given `axis` set to `length`.
    pub fn one(axis: Axis, length: Length) -> Self {
        match axis {
            Axis::Horizontal => Self {
                width: Some(length),
                height: None,
            },
            Axis::Vertical => Self {
                width: None,
                height: Some(length),
            },
        }
    }

    /// Creates a new [`LayoutSize`] with only the given `axis` set to `length`.
    pub fn maybe(axis: Axis, length: Option<Length>) -> Self {
        let Some(length) = length else {
            return Self {
                width: None,
                height: None,
            };
        };
        Self::one(axis, length)
    }

    /// Returns the [`Length`] of the provided `axis`.
    pub const fn length(&self, axis: Axis) -> Option<Length> {
        match axis {
            Axis::Horizontal => self.width,
            Axis::Vertical => self.height,
        }
    }
}
