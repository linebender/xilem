// Copyright 2025 the Xilem Authors
// SPDX-License-Identifier: Apache-2.0

use std::any::TypeId;

use crate::core::{Property, UpdateCtx};
use crate::properties::types::Length;

/// The distance between two adjacent widgets in a [`Flex`](crate::widgets::Flex) or
/// a [`Grid`](crate::widgets::Grid).
///
/// Equivalent to the css [gap] property.
///
/// ## Note on spacers and `Flex`` widgets
///
/// This gap is between any two children, including `Flex`` spacers.
/// As such, using a non-zero gap and also adding may lead to counter-intuitive results.
/// You should usually pick one or the other.
///
/// [gap]: https://developer.mozilla.org/en-US/docs/Web/CSS/gap

#[expect(missing_docs, reason = "field names are self-descriptive")]
#[derive(Clone, Copy, Debug, PartialEq)]
pub struct Gap {
    pub gap: Length,
}

impl Property for Gap {
    fn static_default() -> &'static Self {
        static DEFAULT: Gap = Gap { gap: Length::ZERO };
        &DEFAULT
    }
}

impl Default for Gap {
    fn default() -> Self {
        Self::static_default().clone()
    }
}

impl From<Length> for Gap {
    fn from(gap: Length) -> Self {
        Self { gap }
    }
}

impl Gap {
    /// Zero-sized gap.
    pub const ZERO: Self = Self { gap: Length::ZERO };

    /// Create new `Gap` with given value.
    pub const fn new(gap: Length) -> Self {
        Self { gap }
    }

    /// Helper function to be called in [`Widget::property_changed`](crate::core::Widget::property_changed).
    pub fn prop_changed(ctx: &mut UpdateCtx<'_>, property_type: TypeId) {
        if property_type != TypeId::of::<Self>() {
            return;
        }
        ctx.request_layout();
    }
}
