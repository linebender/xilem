// Copyright 2025 the Xilem Authors
// SPDX-License-Identifier: Apache-2.0

use std::any::TypeId;

use crate::core::UpdateCtx;
use crate::kurbo::Rect;
use crate::peniko::color::{AlphaColor, Srgb};
use crate::properties::types::Gradient;

// TODO - Replace "Background" with "BackgroundColor" and move the gradient case
// to BackgroundImage to match CSS spec.

/// The background color/gradient of a widget.
#[derive(Clone, Debug)]
pub enum Background {
    Color(AlphaColor<Srgb>),
    Gradient(Gradient),
}

impl Background {
    /// Helper function to be called in [`Widget::property_changed`](crate::core::Widget::property_changed).
    pub fn prop_changed(ctx: &mut UpdateCtx<'_>, property_type: TypeId) {
        if property_type != TypeId::of::<Self>() {
            return;
        }
        ctx.request_paint_only();
    }

    pub fn get_peniko_brush_for_rect(&self, rect: Rect) -> crate::peniko::Brush {
        match self {
            Background::Color(color) => (*color).into(),
            Background::Gradient(gradient) => gradient.get_peniko_gradient_for_rect(rect).into(),
        }
    }
}
