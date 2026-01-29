// Copyright 2025 the Xilem Authors
// SPDX-License-Identifier: Apache-2.0

use std::any::TypeId;

use crate::core::{Property, UpdateCtx};
use crate::peniko::color::AlphaColor;
use crate::properties::Background;

/// The background color/gradient a widget takes when toggled on (e.g., for toggle switches).
#[derive(Clone, Debug, PartialEq)]
pub struct ToggledBackground(pub Background);

impl Property for ToggledBackground {
    fn static_default() -> &'static Self {
        static DEFAULT: ToggledBackground =
            ToggledBackground(Background::Color(AlphaColor::TRANSPARENT));
        &DEFAULT
    }
}

impl Default for ToggledBackground {
    fn default() -> Self {
        Self::static_default().clone()
    }
}

impl ToggledBackground {
    /// Helper function to be called in [`Widget::property_changed`](crate::core::Widget::property_changed).
    pub fn prop_changed(ctx: &mut UpdateCtx<'_>, property_type: TypeId) {
        if property_type != TypeId::of::<Self>() {
            return;
        }
        ctx.request_paint_only();
    }
}
