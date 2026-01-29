// Copyright 2025 the Xilem Authors
// SPDX-License-Identifier: Apache-2.0

use crate::core::{HasProperty, Property, Widget};

// Every widget has a corner radius.
impl<W: Widget> HasProperty<CornerRadius> for W {}

/// The radius of a widget's box corners, in logical pixels.
#[expect(missing_docs, reason = "field names are self-descriptive")]
#[derive(Default, Clone, Copy, Debug, PartialEq)]
pub struct CornerRadius {
    pub radius: f64,
}

impl Property for CornerRadius {
    fn static_default() -> &'static Self {
        static DEFAULT: CornerRadius = CornerRadius { radius: 0. };
        &DEFAULT
    }
}

impl CornerRadius {
    /// Creates new `CornerRadius` with given value.
    pub const fn all(radius: f64) -> Self {
        Self { radius }
    }
}
