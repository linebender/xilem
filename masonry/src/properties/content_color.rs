// Copyright 2025 the Xilem Authors
// SPDX-License-Identifier: Apache-2.0

use std::any::TypeId;

use crate::core::{Property, UpdateCtx};
use crate::peniko::color::{AlphaColor, Srgb};

/// The color of a widget's content, often text and text decorations.
///
/// **IMPORTANT:** This property is defined for [`Label`] and [`TextArea`], *not*
/// for widgets embedding them such as [`Button`], [`Checkbox`], [`TextInput`], [`Prose`], etc.
///
/// This property is also defined for [`Spinner`] and [`StepInput`].
///
/// [`Label`]: crate::widgets::Label
/// [`TextArea`]: crate::widgets::TextArea
/// [`Button`]: crate::widgets::Button
/// [`Checkbox`]: crate::widgets::Checkbox
/// [`TextInput`]: crate::widgets::TextInput
/// [`Spinner`]: crate::widgets::Spinner
/// [`StepInput`]: crate::widgets::StepInput
/// [`Prose`]: crate::widgets::Prose
#[expect(missing_docs, reason = "field names are self-descriptive")]
#[derive(Clone, Copy, Debug, PartialEq)]
pub struct ContentColor {
    pub color: AlphaColor<Srgb>,
}

impl Property for ContentColor {
    fn static_default() -> &'static Self {
        static DEFAULT: ContentColor = ContentColor {
            color: AlphaColor::BLACK,
        };
        &DEFAULT
    }
}

impl ContentColor {
    /// Creates new `ContentColor` with given value.
    pub const fn new(color: AlphaColor<Srgb>) -> Self {
        Self { color }
    }
}

/// The color of a widget's content when disabled.
#[derive(Clone, Copy, Debug, PartialEq)]
pub struct DisabledContentColor(pub ContentColor);

impl Property for DisabledContentColor {
    fn static_default() -> &'static Self {
        static DEFAULT: DisabledContentColor = DisabledContentColor(ContentColor {
            color: AlphaColor::BLACK,
        });
        &DEFAULT
    }
}

// ---

impl Default for ContentColor {
    fn default() -> Self {
        *Self::static_default()
    }
}

impl ContentColor {
    /// Helper function to be called in [`Widget::property_changed`](crate::core::Widget::property_changed).
    pub fn prop_changed(ctx: &mut UpdateCtx<'_>, property_type: TypeId) {
        if property_type != TypeId::of::<Self>() {
            return;
        }
        ctx.request_paint_only();
    }
}

// ---

impl Default for DisabledContentColor {
    fn default() -> Self {
        *Self::static_default()
    }
}

impl DisabledContentColor {
    /// Helper function to be called in [`Widget::property_changed`](crate::core::Widget::property_changed).
    pub fn prop_changed(ctx: &mut UpdateCtx<'_>, property_type: TypeId) {
        if property_type != TypeId::of::<Self>() {
            return;
        }
        ctx.request_paint_only();
    }
}
