// Copyright 2025 the Xilem Authors
// SPDX-License-Identifier: Apache-2.0

use std::any::TypeId;

use crate::core::{Property, UpdateCtx};

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

    #[inline(always)]
    fn matches(property_type: TypeId) -> bool {
        property_type == TypeId::of::<Self>()
    }
}

impl CornerRadius {
    /// Creates new `CornerRadius` with given value.
    pub const fn all(radius: f64) -> Self {
        Self { radius }
    }

    /// Helper function to be called in [`Widget::property_changed`](crate::core::Widget::property_changed).
    pub fn prop_changed(ctx: &mut UpdateCtx<'_>, property_type: TypeId) {
        if property_type != TypeId::of::<Self>() {
            return;
        }
        ctx.request_layout();
    }
}
