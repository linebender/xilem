// Copyright 2025 the Xilem Authors
// SPDX-License-Identifier: Apache-2.0

use std::any::TypeId;

use vello::kurbo::{Point, Size, Vec2};

use crate::core::{BoxConstraints, Property, UpdateCtx};
use crate::properties::types::Length;

/// The width of padding between a widget's border and its contents.
#[derive(Clone, Copy, Debug, PartialEq)]
pub struct Padding {
    /// The amount of padding in logical pixels for the left edge.
    pub left: Length,
    /// The amount of padding in logical pixels for the right edge.
    pub right: Length,
    /// The amount of padding in logical pixels for the top edge.
    pub top: Length,
    /// The amount of padding in logical pixels for the bottom edge.
    pub bottom: Length,
}

impl Property for Padding {
    fn static_default() -> &'static Self {
        static DEFAULT: Padding = Padding::ZERO;
        &DEFAULT
    }
}

impl Default for Padding {
    fn default() -> Self {
        *Self::static_default()
    }
}

impl From<Length> for Padding {
    /// Converts the value to a `Padding` object with that amount of padding on all edges.
    fn from(value: Length) -> Self {
        Self::all(value)
    }
}

impl Padding {
    /// A padding of zero for all edges.
    pub const ZERO: Self = Self::all(Length::ZERO);

    /// Constructs a new `Padding` with equal amount of padding for all edges.
    pub const fn all(padding: Length) -> Self {
        Self {
            top: padding,
            bottom: padding,
            left: padding,
            right: padding,
        }
    }

    /// Constructs a new `Padding` with the same amount of padding for the horizontal edges,
    /// and zero padding for the vertical edges.
    pub const fn horizontal(padding: Length) -> Self {
        Self {
            top: Length::ZERO,
            bottom: Length::ZERO,
            left: padding,
            right: padding,
        }
    }

    /// Constructs a new `Padding` with the same amount of padding for the vertical edges,
    /// and zero padding for the horizontal edges.
    pub const fn vertical(padding: Length) -> Self {
        Self {
            top: padding,
            bottom: padding,
            left: Length::ZERO,
            right: Length::ZERO,
        }
    }

    /// Constructs a new `Padding` with the same padding from both vertical edges, then both horizontal edges.
    pub const fn from_vh(vertical: Length, horizontal: Length) -> Self {
        Self {
            top: vertical,
            bottom: vertical,
            left: horizontal,
            right: horizontal,
        }
    }

    /// Constructs a new `Padding` with padding only at the top edge and zero padding for all other edges.
    pub const fn top(padding: Length) -> Self {
        Self {
            top: padding,
            bottom: Length::ZERO,
            left: Length::ZERO,
            right: Length::ZERO,
        }
    }

    /// Constructs a new `Padding` with padding only at the bottom edge and zero padding for all other edges.
    pub const fn bottom(padding: Length) -> Self {
        Self {
            top: Length::ZERO,
            bottom: padding,
            left: Length::ZERO,
            right: Length::ZERO,
        }
    }

    /// Constructs a new `Padding` with padding only at the leleftading edge and zero padding for all other edges.
    pub const fn left(padding: Length) -> Self {
        Self {
            top: Length::ZERO,
            bottom: Length::ZERO,
            left: padding,
            right: Length::ZERO,
        }
    }

    /// Constructs a new `Padding` with padding only at the right edge and zero padding for all other edges.
    pub const fn right(padding: Length) -> Self {
        Self {
            top: Length::ZERO,
            bottom: Length::ZERO,
            left: Length::ZERO,
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
        bc.shrink((
            self.left.value() + self.right.value(),
            self.top.value() + self.bottom.value(),
        ))
    }

    /// Expands the size and raises the baseline by the padding amount.
    ///
    /// Helper function to be called in [`Widget::layout`](crate::core::Widget::layout).
    pub fn layout_up(&self, size: Size, baseline: f64) -> (Size, f64) {
        let size = Size::new(
            size.width + self.left.value() + self.right.value(),
            size.height + self.top.value() + self.bottom.value(),
        );
        let baseline = baseline + self.bottom.value();
        (size, baseline)
    }

    /// Shifts the position by the padding amount.
    ///
    /// Helper function to be called in [`Widget::layout`](crate::core::Widget::layout).
    pub fn place_down(&self, pos: Point) -> Point {
        pos + Vec2::new(self.left.value(), self.top.value())
    }
}
