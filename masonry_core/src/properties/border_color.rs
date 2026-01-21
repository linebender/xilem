// Copyright 2025 the Xilem Authors
// SPDX-License-Identifier: Apache-2.0

use std::any::TypeId;

use crate::core::{Property, UpdateCtx};
use crate::peniko::BrushRef;
use crate::peniko::color::{AlphaColor, Srgb};

/// The color of a widget's border.
#[expect(missing_docs, reason = "field names are self-descriptive")]
#[derive(Clone, Copy, Debug, PartialEq)]
pub struct BorderColor {
    pub color: AlphaColor<Srgb>,
}

/// The color of a widget's border when hovered by a pointer.
#[derive(Clone, Copy, Debug, PartialEq)]
pub struct HoveredBorderColor(pub BorderColor);

/// The color of a widget's border when focused.
#[derive(Clone, Copy, Debug, PartialEq)]
pub struct FocusedBorderColor(pub BorderColor);

// ---

// TODO - The default border color in CSS is `currentcolor`,
// the color text is displayed in.
// Do we want to implement that?

impl Property for BorderColor {
    fn static_default() -> &'static Self {
        static DEFAULT: BorderColor = BorderColor {
            color: AlphaColor::TRANSPARENT,
        };
        &DEFAULT
    }

    #[inline(always)]
    fn matches(property_type: TypeId) -> bool {
        property_type == TypeId::of::<Self>()
    }
}

impl Default for BorderColor {
    fn default() -> Self {
        *Self::static_default()
    }
}

impl<'a> From<&'a BorderColor> for BrushRef<'a> {
    fn from(color: &'a BorderColor) -> Self {
        Self::Solid(color.color)
    }
}

impl BorderColor {
    /// Creates new `BorderColor` with given value.
    pub const fn new(color: AlphaColor<Srgb>) -> Self {
        Self { color }
    }

    /// Helper function to be called in [`Widget::property_changed`](crate::core::Widget::property_changed).
    pub fn prop_changed(ctx: &mut UpdateCtx<'_>, property_type: TypeId) {
        if property_type != TypeId::of::<Self>() {
            return;
        }
        ctx.request_paint_only();
    }
}

// ---

impl Default for HoveredBorderColor {
    fn default() -> Self {
        *Self::static_default()
    }
}

impl Property for HoveredBorderColor {
    fn static_default() -> &'static Self {
        static DEFAULT: HoveredBorderColor = HoveredBorderColor(BorderColor {
            color: AlphaColor::TRANSPARENT,
        });
        &DEFAULT
    }

    #[inline(always)]
    fn matches(property_type: TypeId) -> bool {
        property_type == TypeId::of::<Self>()
    }
}

impl HoveredBorderColor {
    /// Helper function to be called in [`Widget::property_changed`](crate::core::Widget::property_changed).
    pub fn prop_changed(ctx: &mut UpdateCtx<'_>, property_type: TypeId) {
        if property_type != TypeId::of::<Self>() {
            return;
        }
        ctx.request_paint_only();
    }
}

// ---

impl Default for FocusedBorderColor {
    fn default() -> Self {
        *Self::static_default()
    }
}

impl Property for FocusedBorderColor {
    fn static_default() -> &'static Self {
        static DEFAULT: FocusedBorderColor = FocusedBorderColor(BorderColor {
            color: AlphaColor::TRANSPARENT,
        });
        &DEFAULT
    }

    #[inline(always)]
    fn matches(property_type: TypeId) -> bool {
        property_type == TypeId::of::<Self>()
    }
}

impl FocusedBorderColor {
    /// Helper function to be called in [`Widget::property_changed`](crate::core::Widget::property_changed).
    pub fn prop_changed(ctx: &mut UpdateCtx<'_>, property_type: TypeId) {
        if property_type != TypeId::of::<Self>() {
            return;
        }
        ctx.request_paint_only();
    }
}
