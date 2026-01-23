// Copyright 2025 the Xilem Authors
// SPDX-License-Identifier: Apache-2.0

use std::any::TypeId;

use crate::core::{HasProperty, Property, UpdateCtx, Widget};
use crate::kurbo::{Axis, Point, Size, Vec2};
use crate::layout::Length;

// Every widget has padding.
impl<W: Widget> HasProperty<Padding> for W {}

/// The width of padding between a widget's border and its contents.
#[derive(Default, Clone, Copy, Debug, PartialEq)]
pub struct Padding {
    /// The amount of padding in logical pixels for the left edge.
    pub left: f64,
    /// The amount of padding in logical pixels for the right edge.
    pub right: f64,
    /// The amount of padding in logical pixels for the top edge.
    pub top: f64,
    /// The amount of padding in logical pixels for the bottom edge.
    pub bottom: f64,
}

impl Property for Padding {
    fn static_default() -> &'static Self {
        static DEFAULT: Padding = Padding::ZERO;
        &DEFAULT
    }
}

impl From<f64> for Padding {
    /// Converts the value to a `Padding` object with that amount of padding on all edges.
    fn from(value: f64) -> Self {
        Self::all(value)
    }
}

impl Padding {
    /// A padding of zero for all edges.
    pub const ZERO: Self = Self::all(0.);

    /// Creates a new `Padding` with equal amount of padding for all edges.
    pub const fn all(padding: f64) -> Self {
        Self {
            top: padding,
            bottom: padding,
            left: padding,
            right: padding,
        }
    }

    /// Creates a new `Padding` with the same amount of padding for the horizontal edges,
    /// and zero padding for the vertical edges.
    pub const fn horizontal(padding: f64) -> Self {
        Self {
            top: 0.,
            bottom: 0.,
            left: padding,
            right: padding,
        }
    }

    /// Creates a new `Padding` with the same amount of padding for the vertical edges,
    /// and zero padding for the horizontal edges.
    pub const fn vertical(padding: f64) -> Self {
        Self {
            top: padding,
            bottom: padding,
            left: 0.,
            right: 0.,
        }
    }

    /// Creates a new `Padding` with the same padding from both vertical edges, then both horizontal edges.
    pub const fn from_vh(vertical: f64, horizontal: f64) -> Self {
        Self {
            top: vertical,
            bottom: vertical,
            left: horizontal,
            right: horizontal,
        }
    }

    /// Creates a new `Padding` with padding only at the top edge and zero padding for all other edges.
    pub const fn top(padding: f64) -> Self {
        Self {
            top: padding,
            bottom: 0.,
            left: 0.,
            right: 0.,
        }
    }

    /// Creates a new `Padding` with padding only at the bottom edge and zero padding for all other edges.
    pub const fn bottom(padding: f64) -> Self {
        Self {
            top: 0.,
            bottom: padding,
            left: 0.,
            right: 0.,
        }
    }

    /// Creates a new `Padding` with padding only at the leleftading edge and zero padding for all other edges.
    pub const fn left(padding: f64) -> Self {
        Self {
            top: 0.,
            bottom: 0.,
            left: padding,
            right: 0.,
        }
    }

    /// Creates a new `Padding` with padding only at the right edge and zero padding for all other edges.
    pub const fn right(padding: f64) -> Self {
        Self {
            top: 0.,
            bottom: 0.,
            left: 0.,
            right: padding,
        }
    }
}

impl Padding {
    /// Requests layout if this property changed.
    ///
    /// This is called by Masonry during widget properties mutation.
    pub(crate) fn prop_changed(ctx: &mut UpdateCtx<'_>, property_type: TypeId) {
        if property_type != TypeId::of::<Self>() {
            return;
        }
        ctx.request_layout();
    }

    /// Returns the total [`Length`] of this padding on the given `axis`.
    ///
    /// For [`Axis::Horizontal`] it will return the sum of the left and right padding width.
    /// For [`Axis::Vertical`] it will return the sum of the top and bottom padding height.
    pub fn length(&self, axis: Axis) -> Length {
        match axis {
            Axis::Horizontal => Length::px(self.left + self.right),
            Axis::Vertical => Length::px(self.top + self.bottom),
        }
    }

    /// Shrinks the `size` by the padding amount.
    ///
    /// The returned [`Size`] will be non-negative and in device pixels.
    ///
    /// The provided `size` must be in device pixels.
    ///
    /// Helper function to be called in [`Widget::layout`].
    pub fn size_down(&self, size: Size, scale: f64) -> Size {
        let width = (size.width - Length::px(self.left + self.right).dp(scale)).max(0.);
        let height = (size.height - Length::px(self.top + self.bottom).dp(scale)).max(0.);
        Size::new(width, height)
    }

    /// Raises the `baseline` by the padding amount.
    ///
    /// The returned baseline will be in device pixels.
    ///
    /// The provided `baseline` must be in device pixels.
    ///
    /// Helper function to be called in [`Widget::layout`].
    pub fn baseline_up(&self, baseline: f64, scale: f64) -> f64 {
        baseline + Length::px(self.bottom).dp(scale)
    }

    /// Lowers the `baseline` by the padding amount.
    ///
    /// The returned baseline will be in device pixels.
    ///
    /// The provided `baseline` must be in device pixels.
    ///
    /// Helper function to be called in [`Widget::layout`].
    pub fn baseline_down(&self, baseline: f64, scale: f64) -> f64 {
        baseline - Length::px(self.bottom).dp(scale)
    }

    /// Lowers the position by the padding amount.
    ///
    /// The returned [`Point`] will be in device pixels.
    ///
    /// The provided `origin` must be in device pixels.
    ///
    /// Helper function to be called in [`Widget::layout`].
    pub fn origin_down(&self, origin: Point, scale: f64) -> Point {
        let x = Length::px(self.left).dp(scale);
        let y = Length::px(self.top).dp(scale);
        origin + Vec2::new(x, y)
    }
}
