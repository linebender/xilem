// Copyright 2025 the Xilem Authors
// SPDX-License-Identifier: Apache-2.0

use crate::core::{Property, UsesProperty, Widget};
use crate::kurbo::{Axis, Insets, Point, Size, Vec2};
use crate::layout::Length;

// Every widget has padding.
impl<W: Widget> UsesProperty<Padding> for W {}

/// The width of padding between a widget's border and its contents.
#[derive(Default, Clone, Copy, Debug, PartialEq)]
pub struct Padding {
    /// The amount of padding for the left edge.
    pub left: Length,
    /// The amount of padding for the right edge.
    pub right: Length,
    /// The amount of padding for the top edge.
    pub top: Length,
    /// The amount of padding for the bottom edge.
    pub bottom: Length,
}

impl Property for Padding {
    fn static_default() -> &'static Self {
        static DEFAULT: Padding = Padding::ZERO;
        &DEFAULT
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

    /// Creates a new `Padding` with equal amount of padding for all edges.
    pub const fn all(padding: Length) -> Self {
        Self {
            top: padding,
            bottom: padding,
            left: padding,
            right: padding,
        }
    }

    /// Creates a new `Padding` with the same amount of padding for the horizontal edges,
    /// and zero padding for the vertical edges.
    pub const fn horizontal(padding: Length) -> Self {
        Self {
            top: Length::ZERO,
            bottom: Length::ZERO,
            left: padding,
            right: padding,
        }
    }

    /// Creates a new `Padding` with the same amount of padding for the vertical edges,
    /// and zero padding for the horizontal edges.
    pub const fn vertical(padding: Length) -> Self {
        Self {
            top: padding,
            bottom: padding,
            left: Length::ZERO,
            right: Length::ZERO,
        }
    }

    /// Creates a new `Padding` with the same padding from both vertical edges, then both horizontal edges.
    pub const fn from_vh(vertical: Length, horizontal: Length) -> Self {
        Self {
            top: vertical,
            bottom: vertical,
            left: horizontal,
            right: horizontal,
        }
    }

    /// Creates a new `Padding` with padding only at the top edge and zero padding for all other edges.
    pub const fn top(padding: Length) -> Self {
        Self {
            top: padding,
            bottom: Length::ZERO,
            left: Length::ZERO,
            right: Length::ZERO,
        }
    }

    /// Creates a new `Padding` with padding only at the bottom edge and zero padding for all other edges.
    pub const fn bottom(padding: Length) -> Self {
        Self {
            top: Length::ZERO,
            bottom: padding,
            left: Length::ZERO,
            right: Length::ZERO,
        }
    }

    /// Creates a new `Padding` with padding only at the left edge and zero padding for all other edges.
    pub const fn left(padding: Length) -> Self {
        Self {
            top: Length::ZERO,
            bottom: Length::ZERO,
            left: padding,
            right: Length::ZERO,
        }
    }

    /// Creates a new `Padding` with padding only at the right edge and zero padding for all other edges.
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
    /// Returns the total [`Length`] of this padding on the given `axis`.
    ///
    /// For [`Axis::Horizontal`] it will return the sum of the left and right padding width.
    /// For [`Axis::Vertical`] it will return the sum of the top and bottom padding height.
    pub fn length(&self, axis: Axis) -> Length {
        match axis {
            Axis::Horizontal => self.left.saturating_add(self.right),
            Axis::Vertical => self.top.saturating_add(self.bottom),
        }
    }

    /// Expands the `size` by the padding amount.
    ///
    /// The returned [`Size`] will be non-negative and in logical pixels.
    ///
    /// The provided `size` must be in logical pixels.
    ///
    /// Helper function to be called in [`Widget::layout`].
    pub fn size_up(&self, size: Size) -> Size {
        let width = size.width + self.left.get() + self.right.get();
        let height = size.height + self.top.get() + self.bottom.get();
        Size::new(width, height)
    }

    /// Shrinks the `size` by the padding amount.
    ///
    /// The returned [`Size`] will be non-negative and in logical pixels.
    ///
    /// The provided `size` must be in logical pixels.
    ///
    /// Helper function to be called in [`Widget::layout`].
    pub fn size_down(&self, size: Size) -> Size {
        let width = (size.width - self.left.get() - self.right.get()).max(0.);
        let height = (size.height - self.top.get() - self.bottom.get()).max(0.);
        Size::new(width, height)
    }

    /// Returns the [`Insets`] for deriving an area with this padding.
    ///
    /// The returned [`Insets`] will be in logical pixels.
    ///
    /// The provided `insets` must be in logical pixels.
    pub fn insets_up(&self, insets: Insets) -> Insets {
        Insets {
            x0: insets.x0 + self.left.get(),
            y0: insets.y0 + self.top.get(),
            x1: insets.x1 + self.right.get(),
            y1: insets.y1 + self.bottom.get(),
        }
    }

    /// Raises the `baseline` by the padding amount.
    ///
    /// The returned baseline will be in logical pixels.
    ///
    /// The provided `baseline` must be in logical pixels.
    ///
    /// Helper function to be called in [`Widget::layout`].
    pub fn baseline_up(&self, baseline: f64) -> f64 {
        baseline + self.bottom.get()
    }

    /// Lowers the `baseline` by the padding amount.
    ///
    /// The returned baseline will be in logical pixels.
    ///
    /// The provided `baseline` must be in logical pixels.
    ///
    /// Helper function to be called in [`Widget::layout`].
    pub fn baseline_down(&self, baseline: f64) -> f64 {
        baseline - self.bottom.get()
    }

    /// Lowers the position by the padding amount.
    ///
    /// The returned [`Point`] will be in logical pixels.
    ///
    /// The provided `origin` must be in logical pixels.
    ///
    /// Helper function to be called in [`Widget::layout`].
    pub fn origin_down(&self, origin: Point) -> Point {
        origin + Vec2::new(self.left.get(), self.top.get())
    }
}
