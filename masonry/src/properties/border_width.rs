// Copyright 2025 the Xilem Authors
// SPDX-License-Identifier: Apache-2.0

use std::any::TypeId;

use crate::core::{Property, UpdateCtx};
use crate::kurbo::{Axis, Point, RoundedRect, Size, Vec2};
use crate::layout::Length;
use crate::properties::CornerRadius;

/// The width of a widget's border, in logical pixels.
#[expect(missing_docs, reason = "field names are self-descriptive")]
#[derive(Default, Clone, Copy, Debug, PartialEq)]
pub struct BorderWidth {
    pub width: f64,
}

// TODO - To match CSS, we should use a non-zero default width
// and a "border style" of "None".

impl Property for BorderWidth {
    fn static_default() -> &'static Self {
        static DEFAULT: BorderWidth = BorderWidth { width: 0. };
        &DEFAULT
    }
}

impl BorderWidth {
    /// Creates new `BorderWidth` with given value.
    pub const fn all(width: f64) -> Self {
        Self { width }
    }

    /// Helper function to be called in [`Widget::property_changed`](crate::core::Widget::property_changed).
    pub fn prop_changed(ctx: &mut UpdateCtx<'_>, property_type: TypeId) {
        if property_type != TypeId::of::<Self>() {
            return;
        }
        ctx.request_layout();
    }

    /// Returns the [`Length`] on the given `axis`.
    pub fn length(&self, _axis: Axis) -> Length {
        Length::px(self.width * 2.)
    }

    /// Shrinks the `size` by the border width.
    ///
    /// The returned [`Size`] will be non-negative and in device pixels.
    ///
    /// The provided `size` must be in device pixels.
    ///
    /// Helper function to be called in [`Widget::layout`](crate::core::Widget::layout).
    pub fn size_down(&self, size: Size, scale: f64) -> Size {
        let width = (size.width - Length::px(self.width).dp(scale) * 2.).max(0.);
        let height = (size.height - Length::px(self.width).dp(scale) * 2.).max(0.);
        Size::new(width, height)
    }

    /// Raises the `baseline` by the border width.
    ///
    /// The returned baseline will be in device pixels.
    ///
    /// The provided `baseline` must be in device pixels.
    ///
    /// Helper function to be called in [`Widget::layout`](crate::core::Widget::layout).
    pub fn baseline_up(&self, baseline: f64, scale: f64) -> f64 {
        baseline + Length::px(self.width).dp(scale)
    }

    /// Lowers the `baseline` by the border width.
    ///
    /// The returned baseline will be in device pixels.
    ///
    /// The provided `baseline` must be in device pixels.
    ///
    /// Helper function to be called in [`Widget::layout`](crate::core::Widget::layout).
    pub fn baseline_down(&self, baseline: f64, scale: f64) -> f64 {
        baseline - Length::px(self.width).dp(scale)
    }

    /// Lowers the position by the border width.
    ///
    /// The returned [`Point`] will be in device pixels.
    ///
    /// The provided `origin` must be in device pixels.
    ///
    /// Helper function to be called in [`Widget::layout`](crate::core::Widget::layout).
    pub fn origin_down(&self, origin: Point, scale: f64) -> Point {
        let width = Length::px(self.width).dp(scale);
        origin + Vec2::new(width, width)
    }

    /// Creates a rounded rectangle that is inset by the border width.
    ///
    /// Use to display a box's background.
    ///
    /// Helper function to be called in [`Widget::paint`](crate::core::Widget::paint).
    pub fn bg_rect(&self, size: Size, border_radius: &CornerRadius) -> RoundedRect {
        size.to_rect()
            .inset(-self.width)
            .to_rounded_rect((border_radius.radius - self.width).max(0.))
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
