// Copyright 2025 the Xilem Authors
// SPDX-License-Identifier: Apache-2.0

use std::any::TypeId;

use crate::core::{BoxConstraints, Property, UpdateCtx};
use crate::kurbo::{Point, Size, Vec2};

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
    /// Helper function to be called in [`Widget::property_changed`](crate::core::Widget::property_changed).
    pub fn prop_changed(ctx: &mut UpdateCtx<'_>, property_type: TypeId) {
        if property_type != TypeId::of::<Self>() {
            return;
        }
        ctx.request_layout();
    }

    /// Shrinks the box constraints by the padding amount.
    ///
    /// Helper function to be called in [`Widget::layout`](crate::core::Widget::layout).
    pub fn layout_down(&self, bc: BoxConstraints) -> BoxConstraints {
        bc.shrink((self.left + self.right, self.top + self.bottom))
    }

    /// Expands the size and raises the baseline by the padding amount.
    ///
    /// Helper function to be called in [`Widget::layout`](crate::core::Widget::layout).
    pub fn layout_up(&self, size: Size, baseline: f64) -> (Size, f64) {
        let size = Size::new(
            size.width + self.left + self.right,
            size.height + self.top + self.bottom,
        );
        let baseline = baseline + self.bottom;
        (size, baseline)
    }

    /// Shifts the position by the padding amount.
    ///
    /// Helper function to be called in [`Widget::layout`](crate::core::Widget::layout).
    pub fn place_down(&self, pos: Point) -> Point {
        pos + Vec2::new(self.left, self.top)
    }
}
