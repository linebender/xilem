// Copyright 2025 the Xilem Authors
// SPDX-License-Identifier: Apache-2.0

use std::any::TypeId;

use crate::core::{Property, UpdateCtx};
use crate::peniko::color::{AlphaColor, Srgb};

/// The color a spinner is painted with.
#[derive(Clone, Copy, Debug, PartialEq)]
pub struct SpinnerColor(pub AlphaColor<Srgb>);

impl Property for SpinnerColor {
    fn static_default() -> &'static Self {
        static DEFAULT: SpinnerColor = SpinnerColor(AlphaColor::BLACK);
        &DEFAULT
    }
}

impl Default for SpinnerColor {
    fn default() -> Self {
        *Self::static_default()
    }
}

impl SpinnerColor {
    /// Helper function to be called in [`Widget::property_changed`](crate::core::Widget::property_changed).
    pub fn prop_changed(ctx: &mut UpdateCtx<'_>, property_type: TypeId) {
        if property_type != TypeId::of::<Self>() {
            return;
        }
        ctx.request_paint_only();
    }
}
