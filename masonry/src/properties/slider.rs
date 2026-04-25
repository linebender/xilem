// Copyright 2025 the Xilem Authors
// SPDX-License-Identifier: Apache-2.0

use std::any::TypeId;

use crate::core::{Property, UpdateCtx};
use crate::layout::Length;
use crate::peniko::Color;
use crate::theme;

/// The thickness of a slider's track.
#[derive(Default, Clone, Copy, Debug, PartialEq)]
pub struct TrackThickness(pub Length);

impl Property for TrackThickness {
    fn static_default() -> &'static Self {
        static DEFAULT: TrackThickness = TrackThickness(Length::const_px(4.));
        &DEFAULT
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

/// Colors of a slider's track.
#[derive(Clone, Copy, Debug, PartialEq)]
pub struct TrackColor {
    /// Color of the active portion.
    pub active: Color,

    /// Color of the inactive portion.
    pub inactive: Color,
}

impl Property for TrackColor {
    fn static_default() -> &'static Self {
        static DEFAULT: TrackColor = TrackColor {
            active: theme::ACCENT_COLOR,
            inactive: theme::ZYNC_800,
        };
        &DEFAULT
    }
}

impl Default for TrackColor {
    fn default() -> Self {
        *Self::static_default()
    }
}

impl TrackColor {
    /// Helper function to be called in [`Widget::property_changed`](crate::core::Widget::property_changed).
    pub fn prop_changed(ctx: &mut UpdateCtx<'_>, property_type: TypeId) {
        if property_type == TypeId::of::<Self>() {
            ctx.request_paint_only();
        }
    }
}

/// The radius of a slider's thumb.
#[derive(Default, Clone, Copy, Debug, PartialEq)]
pub struct ThumbRadius(pub Length);

impl Property for ThumbRadius {
    fn static_default() -> &'static Self {
        static DEFAULT: ThumbRadius = ThumbRadius(Length::const_px(6.));
        &DEFAULT
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
