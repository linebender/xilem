// Copyright 2025 the Xilem Authors
// SPDX-License-Identifier: Apache-2.0

use crate::core::{Property, UsesProperty, Widget};
use crate::layout::Length;

// Every widget has a corner radius.
impl<W: Widget> UsesProperty<CornerRadius> for W {}

/// The radius of a widget's box corners.
#[expect(missing_docs, reason = "field names are self-descriptive")]
#[derive(Default, Clone, Copy, Debug, PartialEq)]
pub struct CornerRadius {
    pub radius: Length,
}

impl Property for CornerRadius {
    fn static_default() -> &'static Self {
        static DEFAULT: CornerRadius = CornerRadius {
            radius: Length::ZERO,
        };
        &DEFAULT
    }
}

impl CornerRadius {
    /// Creates new `CornerRadius` with given value.
    pub const fn all(radius: Length) -> Self {
        Self { radius }
    }
}
