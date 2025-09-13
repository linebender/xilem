// Copyright 2025 the Xilem Authors
// SPDX-License-Identifier: Apache-2.0

use std::any::TypeId;

use crate::core::{Property, UpdateCtx};
use crate::peniko::color::{AlphaColor, Srgb};

/// The color of a [`TextArea`]'s cursor.
///
/// [`TextArea`]: crate::widgets::TextArea`
#[expect(missing_docs, reason = "field names are self-descriptive")]
#[derive(Clone, Copy, Debug, PartialEq)]
pub struct CaretColor {
    pub color: AlphaColor<Srgb>,
}

impl Property for CaretColor {
    fn static_default() -> &'static Self {
        static DEFAULT: CaretColor = CaretColor {
            color: AlphaColor::BLACK,
        };
        &DEFAULT
    }
}

/// The background color of a [`TextArea`]'s selection.
///
/// [`TextArea`]: crate::widgets::TextArea`
#[expect(missing_docs, reason = "field names are self-descriptive")]
#[derive(Clone, Copy, Debug, PartialEq)]
pub struct SelectionColor {
    pub color: AlphaColor<Srgb>,
}

impl Property for SelectionColor {
    fn static_default() -> &'static Self {
        static DEFAULT: SelectionColor = SelectionColor {
            color: AlphaColor::from_rgb8(70, 130, 255),
        };
        &DEFAULT
    }
}

/// The background color of a [`TextArea`]'s selection when unfocused.
///
/// [`TextArea`]: crate::widgets::TextArea`
#[derive(Clone, Copy, Debug, PartialEq)]
pub struct UnfocusedSelectionColor(pub SelectionColor);

impl Property for UnfocusedSelectionColor {
    fn static_default() -> &'static Self {
        static DEFAULT: UnfocusedSelectionColor = UnfocusedSelectionColor(SelectionColor {
            color: AlphaColor::from_rgb8(176, 176, 176),
        });
        &DEFAULT
    }
}

// ---

impl Default for CaretColor {
    fn default() -> Self {
        *Self::static_default()
    }
}

impl CaretColor {
    /// Helper function to be called in [`Widget::property_changed`](crate::core::Widget::property_changed).
    pub fn prop_changed(ctx: &mut UpdateCtx<'_>, property_type: TypeId) {
        if property_type != TypeId::of::<Self>() {
            return;
        }
        ctx.request_paint_only();
    }
}

// ---

impl Default for SelectionColor {
    fn default() -> Self {
        *Self::static_default()
    }
}

impl SelectionColor {
    /// Helper function to be called in [`Widget::property_changed`](crate::core::Widget::property_changed).
    pub fn prop_changed(ctx: &mut UpdateCtx<'_>, property_type: TypeId) {
        if property_type != TypeId::of::<Self>() {
            return;
        }
        ctx.request_paint_only();
    }
}

// ---

impl Default for UnfocusedSelectionColor {
    fn default() -> Self {
        *Self::static_default()
    }
}

impl UnfocusedSelectionColor {
    /// Helper function to be called in [`Widget::property_changed`](crate::core::Widget::property_changed).
    pub fn prop_changed(ctx: &mut UpdateCtx<'_>, property_type: TypeId) {
        if property_type != TypeId::of::<Self>() {
            return;
        }
        ctx.request_paint_only();
    }
}
