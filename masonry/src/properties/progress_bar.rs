// Copyright 2025 the Xilem Authors
// SPDX-License-Identifier: Apache-2.0

use std::any::TypeId;

use crate::core::{Property, UpdateCtx};
use crate::peniko::color::{AlphaColor, Srgb};

/// The color of a progress bar's "bar".

#[derive(Clone, Copy, Debug, PartialEq)]
pub struct BarColor(pub AlphaColor<Srgb>);

impl Property for BarColor {
    fn static_default() -> &'static Self {
        static DEFAULT: BarColor = BarColor(AlphaColor::BLACK);
        &DEFAULT
    }

    #[inline(always)]
    fn matches(property_type: TypeId) -> bool {
        property_type == TypeId::of::<Self>()
    }
}

impl Default for BarColor {
    fn default() -> Self {
        *Self::static_default()
    }
}

impl BarColor {
    /// Helper function to be called in [`Widget::property_changed`](crate::core::Widget::property_changed).
    pub fn prop_changed(ctx: &mut UpdateCtx<'_>, property_type: TypeId) {
        if property_type != TypeId::of::<Self>() {
            return;
        }
        ctx.request_paint_only();
    }
}
