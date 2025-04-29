// Copyright 2025 the Xilem Authors
// SPDX-License-Identifier: Apache-2.0

use std::any::TypeId;

use crate::core::{BoxConstraints, Property, UpdateCtx};
use crate::kurbo::{Point, RoundedRect, Size, Vec2};
use crate::properties::CornerRadius;

/// The width of a widget's border, in logical pixels.
#[expect(missing_docs, reason = "obvious")]
#[derive(Default, Clone, Copy, Debug)]
pub struct BorderWidth {
    pub width: f64,
}

// TODO - To match CSS, we should use a non-zero default width
// and a "border style" of "None".

impl Property for BorderWidth {}

impl BorderWidth {
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
        bc.shrink((self.width * 2., self.width * 2.))
    }

    /// Expands the size and raises the baseline by the border width.
    ///
    /// Helper function to be called in [`Widget::layout`](crate::core::Widget::layout).
    pub fn layout_up(&self, size: Size, baseline: f64) -> (Size, f64) {
        let size = Size::new(size.width + self.width * 2., size.height + self.width * 2.);
        let baseline = baseline + self.width;
        (size, baseline)
    }

    /// Shifts the position by the border width.
    ///
    /// Helper function to be called in [`Widget::layout`](crate::core::Widget::layout).
    pub fn place_down(&self, pos: Point) -> Point {
        pos + Vec2::new(self.width, self.width)
    }

    /// Creates a rounded rectangle that is inset by the border width.
    ///
    /// Use to display a box's background.
    ///
    /// Helper function to be called in [`Widget::paint`](crate::core::Widget::paint).
    pub fn bg_rect(&self, size: Size, border_radius: &CornerRadius) -> RoundedRect {
        size.to_rect()
            .inset(-self.width)
            .to_rounded_rect(border_radius.radius - self.width)
    }

    /// Creates a rounded rectangle that is inset by half the border width.
    ///
    /// Use to display a box's border.
    ///
    /// Helper function to be called in [`Widget::paint`](crate::core::Widget::paint).
    pub fn border_rect(&self, size: Size, border_radius: &CornerRadius) -> RoundedRect {
        size.to_rect()
            .inset(-self.width / 2.0)
            .to_rounded_rect(border_radius.radius)
    }
}
