// Copyright 2025 the Xilem Authors
// SPDX-License-Identifier: Apache-2.0

use std::any::TypeId;

use crate::core::UpdateCtx;
use crate::peniko::color::{AlphaColor, Srgb};

/// The background color of a widget.
#[derive(Clone, Copy, Debug)]
pub struct BorderColor {
    pub color: AlphaColor<Srgb>,
}

impl BorderColor {
    pub(crate) fn prop_changed(ctx: &mut UpdateCtx<'_>, property_type: TypeId) {
        if property_type != TypeId::of::<Self>() {
            return;
        }
        ctx.request_paint_only();
    }
}
