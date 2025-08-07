// Copyright 2025 the Xilem Authors
// SPDX-License-Identifier: Apache-2.0

use std::any::TypeId;

use crate::{
    core::{Property, UpdateCtx},
    properties::types::Length,
};

/// The radius of a widget's box corners, in logical pixels.
#[expect(missing_docs, reason = "field names are self-descriptive")]
#[derive(Clone, Copy, Debug, PartialEq)]
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

impl Default for CornerRadius {
    fn default() -> Self {
        *Self::static_default()
    }
}

impl CornerRadius {
    /// Create new `CornerRadius` with given value.
    pub const fn all(radius: Length) -> Self {
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
