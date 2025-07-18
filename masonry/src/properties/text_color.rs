// Copyright 2025 the Xilem Authors
// SPDX-License-Identifier: Apache-2.0

use std::any::TypeId;

use crate::core::{Property, UpdateCtx};
use crate::peniko::color::{AlphaColor, Srgb};

/// The color of a widget's text and text decorations.
///
/// **IMPORTANT:** This property is only defined for [`Label`] and [`TextArea`], *not*
/// for widgets embedding them such as [`Button`], [`Checkbox`], [`TextInput`], [`Prose`], etc.
///
/// [`Label`]: crate::widgets::Label
/// [`TextArea`]: crate::widgets::TextArea
/// [`Button`]: crate::widgets::Button
/// [`Checkbox`]: crate::widgets::Checkbox
/// [`TextInput`]: crate::widgets::TextInput
/// [`Prose`]: crate::widgets::Prose
#[expect(missing_docs, reason = "field names are self-descriptive")]
#[derive(Clone, Copy, Debug, PartialEq)]
pub struct TextColor {
    pub color: AlphaColor<Srgb>,
}

impl Property for TextColor {
    fn static_default() -> &'static Self {
        static DEFAULT: TextColor = TextColor {
            color: AlphaColor::BLACK,
        };
        &DEFAULT
    }
}

impl TextColor {
    /// Create new `TextColor` with given value.
    pub fn new(color: AlphaColor<Srgb>) -> Self {
        Self { color }
    }
}

/// The color of a widget's text and text decorations when disabled.
#[derive(Clone, Copy, Debug, PartialEq)]
pub struct DisabledTextColor(pub TextColor);

impl Property for DisabledTextColor {
    fn static_default() -> &'static Self {
        static DEFAULT: DisabledTextColor = DisabledTextColor(TextColor {
            color: AlphaColor::BLACK,
        });
        &DEFAULT
    }
}

// ---

impl Default for TextColor {
    fn default() -> Self {
        *Self::static_default()
    }
}

impl TextColor {
    /// Helper function to be called in [`Widget::property_changed`](crate::core::Widget::property_changed).
    pub fn prop_changed(ctx: &mut UpdateCtx<'_>, property_type: TypeId) {
        if property_type != TypeId::of::<Self>() {
            return;
        }
        ctx.request_paint_only();
    }
}

// ---

impl Default for DisabledTextColor {
    fn default() -> Self {
        *Self::static_default()
    }
}

impl DisabledTextColor {
    /// Helper function to be called in [`Widget::property_changed`](crate::core::Widget::property_changed).
    pub fn prop_changed(ctx: &mut UpdateCtx<'_>, property_type: TypeId) {
        if property_type != TypeId::of::<Self>() {
            return;
        }
        ctx.request_paint_only();
    }
}
