// Copyright 2025 the Xilem Authors
// SPDX-License-Identifier: Apache-2.0

use std::any::TypeId;

use crate::core::{Property, UpdateCtx};
use crate::peniko::Color;
use crate::theme;

/// The thickness of a slider's track, in logical pixels.
#[derive(Default, Clone, Copy, Debug, PartialEq)]
pub struct TrackThickness(pub f64);

impl Property for TrackThickness {
    fn static_default() -> &'static Self {
        static DEFAULT: TrackThickness = TrackThickness(4.);
        &DEFAULT
    }

    #[inline(always)]
    fn matches(property_type: TypeId) -> bool {
        property_type == TypeId::of::<Self>()
    }
}

impl TrackThickness {
    /// Helper function to be called in [`Widget::property_changed`](crate::core::Widget::property_changed).
    pub fn prop_changed(ctx: &mut UpdateCtx<'_>, property_type: TypeId) {
        if property_type == TypeId::of::<Self>() {
            ctx.request_layout();
        }
    }
}

/// The radius of a slider's thumb, in logical pixels.
#[derive(Default, Clone, Copy, Debug, PartialEq)]
pub struct ThumbRadius(pub f64);

impl Property for ThumbRadius {
    fn static_default() -> &'static Self {
        static DEFAULT: ThumbRadius = ThumbRadius(6.);
        &DEFAULT
    }

    #[inline(always)]
    fn matches(property_type: TypeId) -> bool {
        property_type == TypeId::of::<Self>()
    }
}

impl ThumbRadius {
    /// Helper function to be called in [`Widget::property_changed`](crate::core::Widget::property_changed).
    pub fn prop_changed(ctx: &mut UpdateCtx<'_>, property_type: TypeId) {
        if property_type == TypeId::of::<Self>() {
            ctx.request_layout();
        }
    }
}

/// The color of a slider's thumb.

#[derive(Clone, Copy, Debug, PartialEq)]
pub struct ThumbColor(pub Color);

impl Property for ThumbColor {
    fn static_default() -> &'static Self {
        static DEFAULT: ThumbColor = ThumbColor(theme::TEXT_COLOR);
        &DEFAULT
    }

    #[inline(always)]
    fn matches(property_type: TypeId) -> bool {
        property_type == TypeId::of::<Self>()
    }
}

impl Default for ThumbColor {
    fn default() -> Self {
        *Self::static_default()
    }
}

impl ThumbColor {
    /// Helper function to be called in [`Widget::property_changed`](crate::core::Widget::property_changed).
    pub fn prop_changed(ctx: &mut UpdateCtx<'_>, property_type: TypeId) {
        if property_type == TypeId::of::<Self>() {
            ctx.request_paint_only();
        }
    }
}
