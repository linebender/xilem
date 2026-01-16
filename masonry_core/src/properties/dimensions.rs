// Copyright 2025 the Xilem Authors
// SPDX-License-Identifier: Apache-2.0

use std::any::TypeId;

use vello::kurbo::Axis;

use crate::{
    core::{HasProperty, Property, UpdateCtx, Widget},
    layout::{Dim, Length},
};

// Every widget has dimensions.
impl<W: Widget> HasProperty<Dimensions> for W {}

/// The size of the widget, including borders and padding.
///
/// Generally this is meant to be used as a write-only property.
/// Masonry will automatically resolve the dimensions to actual numbers during the layout pass.
#[derive(Copy, Clone, Debug, Default, PartialEq)]
pub struct Dimensions {
    width: Dim,
    height: Dim,
}

impl Property for Dimensions {
    fn static_default() -> &'static Self {
        static DEFAULT: Dimensions = Dimensions::AUTO;
        &DEFAULT
    }
}

impl From<Dim> for Dimensions {
    fn from(dim: Dim) -> Self {
        Self::new(dim, dim)
    }
}

impl From<(Dim, Dim)> for Dimensions {
    fn from(dims: (Dim, Dim)) -> Self {
        Self::new(dims.0, dims.1)
    }
}

impl From<Length> for Dimensions {
    fn from(value: Length) -> Self {
        Self::fixed(value, value)
    }
}

impl From<(Length, Length)> for Dimensions {
    fn from(value: (Length, Length)) -> Self {
        Self::fixed(value.0, value.1)
    }
}

impl From<(Length, Dim)> for Dimensions {
    fn from(value: (Length, Dim)) -> Self {
        Self::new(Dim::Fixed(value.0), value.1)
    }
}

impl From<(Dim, Length)> for Dimensions {
    fn from(value: (Dim, Length)) -> Self {
        Self::new(value.0, Dim::Fixed(value.1))
    }
}

impl Dimensions {
    /// Automatically sized dimensions.
    ///
    /// See [`Dim::Auto`] for details.
    pub const AUTO: Self = Self::new(Dim::Auto, Dim::Auto);

    /// Stretch to the full context size.
    ///
    /// See [`Dim::Stretch`] for details.
    pub const STRETCH: Self = Self::new(Dim::Stretch, Dim::Stretch);

    /// Minimum preferred size.
    ///
    /// See [`Dim::MinContent`] for details.
    pub const MIN: Self = Self::new(Dim::MinContent, Dim::MinContent);

    /// Maximum preferred size.
    ///
    /// See [`Dim::MaxContent`] for details.
    pub const MAX: Self = Self::new(Dim::MaxContent, Dim::MaxContent);

    /// Fit into the context size.
    ///
    /// See [`Dim::FitContent`] for details.
    pub const FIT: Self = Self::new(Dim::FitContent, Dim::FitContent);

    /// Creates new [`Dimensions`].
    pub const fn new(width: Dim, height: Dim) -> Self {
        Self { width, height }
    }

    /// Creates new fixed [`Length`] [`Dimensions`].
    pub const fn fixed(width: Length, height: Length) -> Self {
        Self {
            width: Dim::Fixed(width),
            height: Dim::Fixed(height),
        }
    }

    /// Creates new [`Dimensions`] with the given `width_ratio` and `height_ratio`.
    ///
    /// The ratio values are multipliers, e.g. `0.5` means half the context size.
    ///
    /// The ratio values must be finite and non-negative.
    ///
    /// See [`Dim::Ratio`] for details.
    pub const fn ratio(width_ratio: f64, height_ratio: f64) -> Self {
        Self {
            width: Dim::Ratio(width_ratio),
            height: Dim::Ratio(height_ratio),
        }
    }

    /// Creates new [`Dimensions`] with width set to `dim`.
    ///
    /// Height is [`Dim::Auto`].
    pub fn width(dim: impl Into<Dim>) -> Self {
        Self {
            width: dim.into(),
            height: Dim::Auto,
        }
    }

    /// Creates new [`Dimensions`] with height set to `dim`.
    ///
    /// Width is [`Dim::Auto`].
    pub fn height(dim: impl Into<Dim>) -> Self {
        Self {
            width: Dim::Auto,
            height: dim.into(),
        }
    }

    /// Returns [`Dimensions`] with `axis` changed to `dim`.
    pub fn with(self, axis: Axis, dim: impl Into<Dim>) -> Self {
        match axis {
            Axis::Horizontal => self.with_width(dim),
            Axis::Vertical => self.with_height(dim),
        }
    }

    /// Returns [`Dimensions`] with the width changed to `dim`.
    pub fn with_width(mut self, dim: impl Into<Dim>) -> Self {
        self.width = dim.into();
        self
    }

    /// Returns [`Dimensions`] with the height changed to `dim`.
    pub fn with_height(mut self, dim: impl Into<Dim>) -> Self {
        self.height = dim.into();
        self
    }

    /// Returns the [`Dim`] of the provided `axis`.
    pub const fn dim(&self, axis: Axis) -> Dim {
        match axis {
            Axis::Horizontal => self.width,
            Axis::Vertical => self.height,
        }
    }

    /// Requests layout if this property changed.
    ///
    /// This is called by Masonry during widget properties mutation.
    pub(crate) fn prop_changed(ctx: &mut UpdateCtx<'_>, property_type: TypeId) {
        if property_type != TypeId::of::<Self>() {
            return;
        }
        ctx.request_layout();
    }
}
