// Copyright 2025 the Xilem Authors
// SPDX-License-Identifier: Apache-2.0

use crate::core::{Property, UsesProperty, Widget};
use kurbo::RoundedRectRadii;

// Every widget has a corner radius.
impl<W: Widget> UsesProperty<CornerRadius> for W {}

/// The radius of a widget's box corners, in logical pixels.
#[expect(missing_docs, reason = "field names are self-descriptive")]
#[derive(Default, Clone, Copy, Debug, PartialEq)]
pub struct CornerRadius {
    pub top_left: f64,
    pub top_right: f64,
    pub bottom_left: f64,
    pub bottom_right: f64,
}

impl Property for CornerRadius {
    fn static_default() -> &'static Self {
        static DEFAULT: CornerRadius = CornerRadius {
            top_left: 0.0,
            top_right: 0.0,
            bottom_left: 0.0,
            bottom_right: 0.0,
        };
        &DEFAULT
    }
}

impl CornerRadius {
    /// Creates new `CornerRadius` with given value.
    pub const fn all(radius: f64) -> Self {
        Self {
            top_left: radius,
            top_right: radius,
            bottom_left: radius,
            bottom_right: radius,
        }
    }

    /// Creates new `CornerRadius` with given value.
    pub const fn top(radius: f64) -> Self {
        Self {
            top_left: radius,
            top_right: radius,
            bottom_left: 0.0,
            bottom_right: 0.0,
        }
    }

    /// Creates new `CornerRadius` with given value.
    pub const fn bottom(radius: f64) -> Self {
        Self {
            top_left: 0.0,
            top_right: 0.0,
            bottom_left: radius,
            bottom_right: radius,
        }
    }

    /// Creates new `CornerRadius` with given value.
    pub const fn right(radius: f64) -> Self {
        Self {
            top_left: 0.0,
            bottom_right: radius,
            top_right: radius,
            bottom_left: 0.0,
        }
    }

    /// Creates new `CornerRadius` with given value.
    pub const fn left(radius: f64) -> Self {
        Self {
            top_left: radius,
            bottom_right: 0.0,
            top_right: 0.0,
            bottom_left: radius,
        }
    }
}

impl Into<RoundedRectRadii> for CornerRadius {
    fn into(self) -> RoundedRectRadii {
        RoundedRectRadii {
            top_left: self.top_left,
            top_right: self.top_right,
            bottom_left: self.bottom_left,
            bottom_right: self.bottom_right,
        }
    }
}
