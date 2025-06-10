// Copyright 2025 the Xilem Authors
// SPDX-License-Identifier: Apache-2.0

use std::any::TypeId;

use crate::core::{Property, UpdateCtx};
use crate::peniko::color::{AlphaColor, Srgb};

/// The color of a checkbox's "check" icon.
#[expect(missing_docs, reason = "obvious")]
#[derive(Clone, Copy, Debug, PartialEq)]
pub struct CheckmarkColor {
    pub color: AlphaColor<Srgb>,
}

impl Property for CheckmarkColor {
    fn static_default() -> &'static Self {
        static DEFAULT: CheckmarkColor = CheckmarkColor {
            color: AlphaColor::BLACK,
        };
        &DEFAULT
    }
}

/// The color of a checkbox's "check" icon when disabled.
#[derive(Clone, Copy, Debug, PartialEq)]
pub struct DisabledCheckmarkColor(pub CheckmarkColor);

impl Property for DisabledCheckmarkColor {
    fn static_default() -> &'static Self {
        static DEFAULT: DisabledCheckmarkColor = DisabledCheckmarkColor(CheckmarkColor {
            color: AlphaColor::BLACK,
        });
        &DEFAULT
    }
}

/// The with the stroke with which a checkbox's "check" icon is drawn.
#[expect(missing_docs, reason = "obvious")]
#[derive(Clone, Copy, Debug, PartialEq)]
pub struct CheckmarkStrokeWidth {
    pub width: f64,
}

impl Property for CheckmarkStrokeWidth {
    fn static_default() -> &'static Self {
        static DEFAULT: CheckmarkStrokeWidth = CheckmarkStrokeWidth { width: 1. };
        &DEFAULT
    }
}

// ---

impl Default for CheckmarkColor {
    fn default() -> Self {
        *Self::static_default()
    }
}

impl CheckmarkColor {
    /// Helper function to be called in [`Widget::property_changed`](crate::core::Widget::property_changed).
    pub fn prop_changed(ctx: &mut UpdateCtx<'_>, property_type: TypeId) {
        if property_type != TypeId::of::<Self>() {
            return;
        }
        ctx.request_paint_only();
    }
}

// ---

impl Default for DisabledCheckmarkColor {
    fn default() -> Self {
        *Self::static_default()
    }
}

impl DisabledCheckmarkColor {
    /// Helper function to be called in [`Widget::property_changed`](crate::core::Widget::property_changed).
    pub fn prop_changed(ctx: &mut UpdateCtx<'_>, property_type: TypeId) {
        if property_type != TypeId::of::<Self>() {
            return;
        }
        ctx.request_paint_only();
    }
}

// ---

impl Default for CheckmarkStrokeWidth {
    fn default() -> Self {
        *Self::static_default()
    }
}

impl CheckmarkStrokeWidth {
    /// Helper function to be called in [`Widget::property_changed`](crate::core::Widget::property_changed).
    pub fn prop_changed(ctx: &mut UpdateCtx<'_>, property_type: TypeId) {
        if property_type != TypeId::of::<Self>() {
            return;
        }
        ctx.request_paint_only();
    }
}
