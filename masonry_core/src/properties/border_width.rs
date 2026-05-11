// Copyright 2025 the Xilem Authors
// SPDX-License-Identifier: Apache-2.0

use crate::core::{Property, UsesProperty, Widget};
use crate::kurbo::{Axis, Insets, Point, Rect, RoundedRect, Size, Vec2};
use crate::layout::Length;
use crate::properties::CornerRadius;

// Every widget has a border width.
impl<W: Widget> UsesProperty<BorderWidth> for W {}

/// The width of a widget's border.
#[expect(missing_docs, reason = "field names are self-descriptive")]
#[derive(Default, Clone, Copy, Debug, PartialEq)]
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

impl BorderWidth {
    /// Creates new `BorderWidth` with given value.
    pub const fn all(width: Length) -> Self {
        Self { width }
    }

    /// Returns the total [`Length`] of this border on the given `axis`.
    ///
    /// For [`Axis::Horizontal`] it will return the sum of the left and right border width.
    /// For [`Axis::Vertical`] it will return the sum of the top and bottom border height.
    pub fn length(&self, _axis: Axis) -> Length {
        self.width.saturating_add(self.width)
    }

    /// Expands the `size` by the border width.
    ///
    /// The returned [`Size`] will be non-negative and in logical pixels.
    ///
    /// The provided `size` must be in logical pixels.
    ///
    /// Helper function to be called in [`Widget::layout`].
    pub fn size_up(&self, size: Size) -> Size {
        let border_width = self.width.get();
        let width = size.width + border_width * 2.;
        let height = size.height + border_width * 2.;
        Size::new(width, height)
    }

    /// Shrinks the `size` by the border width.
    ///
    /// The returned [`Size`] will be non-negative and in logical pixels.
    ///
    /// The provided `size` must be in logical pixels.
    ///
    /// Helper function to be called in [`Widget::layout`].
    pub fn size_down(&self, size: Size) -> Size {
        let border_width = self.width.get();
        let width = (size.width - border_width * 2.).max(0.);
        let height = (size.height - border_width * 2.).max(0.);
        Size::new(width, height)
    }

    /// Returns the [`Insets`] for deriving an area with this border.
    ///
    /// The returned [`Insets`] will be in logical pixels.
    ///
    /// The provided `insets` must be in logical pixels.
    pub fn insets_up(&self, insets: Insets) -> Insets {
        let border_width = self.width.get();
        Insets {
            x0: insets.x0 + border_width,
            y0: insets.y0 + border_width,
            x1: insets.x1 + border_width,
            y1: insets.y1 + border_width,
        }
    }

    /// Raises the `baseline` by the border width.
    ///
    /// The returned baseline will be in logical pixels.
    ///
    /// The provided `baseline` must be in logical pixels.
    ///
    /// Helper function to be called in [`Widget::layout`].
    pub fn baseline_up(&self, baseline: f64) -> f64 {
        baseline + self.width.get()
    }

    /// Lowers the `baseline` by the border width.
    ///
    /// The returned baseline will be in logical pixels.
    ///
    /// The provided `baseline` must be in logical pixels.
    ///
    /// Helper function to be called in [`Widget::layout`].
    pub fn baseline_down(&self, baseline: f64) -> f64 {
        baseline - self.width.get()
    }

    /// Lowers the position by the border width.
    ///
    /// The returned [`Point`] will be in logical pixels.
    ///
    /// The provided `origin` must be in logical pixels.
    ///
    /// Helper function to be called in [`Widget::layout`].
    pub fn origin_down(&self, origin: Point) -> Point {
        let border_width = self.width.get();
        origin + Vec2::new(border_width, border_width)
    }

    /// Creates a rounded rectangle that is inset by the border width.
    ///
    /// Use to display a box's background.
    ///
    /// Helper function to be called in [`Widget::paint`].
    pub fn bg_rect(&self, border_box: Rect, border_radius: &CornerRadius) -> RoundedRect {
        let border_width = self.width.get();
        border_box
            .inset(-border_width)
            .to_rounded_rect(border_radius.radius.saturating_sub(self.width).get())
    }

    /// Creates a rounded rectangle that is inset by half the border width.
    ///
    /// Use to display a box's border.
    ///
    /// Helper function to be called in [`Widget::paint`].
    pub fn border_rect(&self, border_box: Rect, border_radius: &CornerRadius) -> RoundedRect {
        let border_width = self.width.get();
        border_box
            .inset(-border_width / 2.0)
            .to_rounded_rect(border_radius.radius.get())
    }
}
