// Copyright 2025 the Xilem Authors
// SPDX-License-Identifier: Apache-2.0

use std::any::TypeId;

use crate::core::{Property, UpdateCtx};
use crate::peniko::color::{AlphaColor, Srgb};

/// The color of a widget's border.
#[expect(missing_docs, reason = "obvious")]
#[derive(Clone, Copy, Debug)]
pub struct BorderColor {
    pub color: AlphaColor<Srgb>,
}

impl Property for BorderColor {}

// TODO - The default border color in CSS is `currentcolor`,
// the color text is displayed in.
// Do we want to implement that?

impl Default for BorderColor {
    fn default() -> Self {
        Self {
            color: AlphaColor::from_rgba8(0, 0, 0, 0),
        }
    }
}

impl BorderColor {
    /// Helper function to be called in [`Widget::property_changed`](crate::core::Widget::property_changed).
    pub fn prop_changed(ctx: &mut UpdateCtx<'_>, property_type: TypeId) {
        if property_type != TypeId::of::<Self>() {
            return;
        }
        ctx.request_paint_only();
    }
}
