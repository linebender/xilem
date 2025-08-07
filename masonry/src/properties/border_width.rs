// Copyright 2025 the Xilem Authors
// SPDX-License-Identifier: Apache-2.0

use std::any::TypeId;

use vello::kurbo::{Point, RoundedRect, Size, Vec2};

use crate::core::{BoxConstraints, Property, UpdateCtx};
use crate::properties::CornerRadius;
use crate::properties::types::Length;

/// The width of a widget's border, in logical pixels.
#[expect(missing_docs, reason = "field names are self-descriptive")]
#[derive(Clone, Copy, Debug, PartialEq)]
pub struct BorderWidth {
    pub width: Length,
}

// TODO - To match CSS, we should use a non-zero default width
// and a "border style" of "None".

impl Property for BorderWidth {
    fn static_default() -> &'static Self {
        static DEFAULT: BorderWidth = BorderWidth {
            width: Length::ZERO,
        };
        &DEFAULT
    }
}

impl Default for BorderWidth {
    fn default() -> Self {
        *Self::static_default()
    }
}

impl BorderWidth {
    /// Create new `BorderWidth` with given value.
    pub const fn all(width: Length) -> Self {
        Self { width }
    }

    /// Helper function to be called in [`Widget::property_changed`](crate::core::Widget::property_changed).
    pub fn prop_changed(ctx: &mut UpdateCtx<'_>, property_type: TypeId) {
        if property_type != TypeId::of::<Self>() {
            return;
        }
        ctx.request_layout();
    }

    /// Shrinks the box constraints by the border width.
    ///
    /// Helper function to be called in [`Widget::layout`](crate::core::Widget::layout).
    pub fn layout_down(&self, bc: BoxConstraints) -> BoxConstraints {
        bc.shrink((self.width.value() * 2., self.width.value() * 2.))
    }

    /// Expands the size and raises the baseline by the border width.
    ///
    /// Helper function to be called in [`Widget::layout`](crate::core::Widget::layout).
    pub fn layout_up(&self, size: Size, baseline: f64) -> (Size, f64) {
        let size = Size::new(
            size.width + self.width.value() * 2.,
            size.height + self.width.value() * 2.,
        );
        let baseline = baseline + self.width.value();
        (size, baseline)
    }

    /// Shifts the position by the border width.
    ///
    /// Helper function to be called in [`Widget::layout`](crate::core::Widget::layout).
    pub fn place_down(&self, pos: Point) -> Point {
        pos + Vec2::new(self.width.value(), self.width.value())
    }

    /// Creates a rounded rectangle that is inset by the border width.
    ///
    /// Use to display a box's background.
    ///
    /// Helper function to be called in [`Widget::paint`](crate::core::Widget::paint).
    pub fn bg_rect(&self, size: Size, border_radius: &CornerRadius) -> RoundedRect {
        size.to_rect()
            .inset(-self.width.value())
            .to_rounded_rect((border_radius.radius.value() - self.width.value()).max(0.))
    }

    /// Creates a rounded rectangle that is inset by half the border width.
    ///
    /// Use to display a box's border.
    ///
    /// Helper function to be called in [`Widget::paint`](crate::core::Widget::paint).
    pub fn border_rect(&self, size: Size, border_radius: &CornerRadius) -> RoundedRect {
        size.to_rect()
            .inset(-self.width.value() / 2.0)
            .to_rounded_rect(border_radius.radius.value())
    }
}
