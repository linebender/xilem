// Copyright 2025 the Xilem Authors
// SPDX-License-Identifier: Apache-2.0

use crate::kurbo::{Point, Rect};
use crate::peniko::color::{ColorSpaceTag, HueDirection};
use crate::peniko::{ColorStops, ColorStopsSource, Extend};

/// Properties for the supported [`Gradient`] types.
///
/// This mirrors [`peniko::GradientKind`](crate::peniko::GradientKind),
/// but uses a layout-invariant representation: instead of saying
/// "The gradient goes from point A to point B", we declare things like
/// "The gradient has angle X", and A and B and computed dynamically from widget layout.
#[derive(Clone, Debug)]
pub enum GradientShape {
    /// Gradient that transitions between two or more colors along a line.
    ///
    /// This is interpreted like [`linear-gradient()`] in CSS.
    ///
    /// [`linear-gradient()`]: https://drafts.csswg.org/css-images-3/#linear-gradient-syntax
    Linear {
        /// The angle defining the gradient line's direction, in radians.
        /// Zero points upwards and positive angles represent clockwise rotation.
        angle: f64,
    },
}

/// Definition of a gradient that transitions between two or more colors.
///
/// This mirrors [`peniko::Gradient`](crate::peniko::Gradient),
/// but uses a layout-invariant representation: instead of saying
/// "The gradient goes from point A to point B", we declare things like
/// "The gradient has angle X", and A and B and computed dynamically from widget layout.
#[derive(Clone, Debug)]
pub struct Gradient {
    /// Shape and coordinates of the gradient.
    pub shape: GradientShape,
    /// Extend mode.
    pub extend: Extend,
    /// The color space to be used for interpolation.
    ///
    /// The colors in the color stops will be converted to this color space.
    ///
    /// This defaults to [sRGB](ColorSpaceTag::Srgb).
    pub interpolation_cs: ColorSpaceTag,
    /// When interpolating within a cylindrical color space, the direction for the hue.
    ///
    /// This is interpreted as described in [CSS Color Module Level 4 § 12.4].
    ///
    /// [CSS Color Module Level 4 § 12.4]: https://drafts.csswg.org/css-color/#hue-interpolation
    pub hue_direction: HueDirection,
    /// Color stop collection.
    pub stops: ColorStops,
}

impl Gradient {
    /// Creates a [`Linear`](GradientShape::Linear) gradient.
    ///
    /// `angle` is in radians, with zero pointing upwards.
    pub fn new_linear(angle: f64) -> Self {
        Self {
            shape: GradientShape::Linear { angle },
            extend: Default::default(),
            interpolation_cs: ColorSpaceTag::Srgb,
            hue_direction: Default::default(),
            stops: Default::default(),
        }
    }

    /// Builder method to set color stops on the gradient.
    pub fn with_stops(mut self, stops: impl ColorStopsSource) -> Self {
        self.stops.clear();
        stops.collect_stops(&mut self.stops);
        self
    }

    /// Returns gradient brush covering the given Rect.
    ///
    /// This matches the CSS spec for [`linear-gradient()`](https://drafts.csswg.org/css-images-3/#linear-gradient-syntax).
    pub fn get_peniko_gradient_for_rect(&self, rect: Rect) -> crate::peniko::Gradient {
        crate::peniko::Gradient {
            kind: self.shape.get_peniko_kind_for_rect(rect),
            extend: self.extend,
            interpolation_cs: self.interpolation_cs,
            hue_direction: self.hue_direction,
            stops: self.stops.clone(),
        }
    }
}

impl GradientShape {
    /// Returns gradient coordinates for a gradient covering the given Rect.
    ///
    /// This matches the CSS spec for [`linear-gradient()`](https://drafts.csswg.org/css-images-3/#linear-gradient-syntax).
    pub fn get_peniko_kind_for_rect(&self, rect: Rect) -> crate::peniko::GradientKind {
        match self {
            Self::Linear { angle } => Self::get_peniko_linear_for_rect(*angle, rect),
        }
    }

    fn get_peniko_linear_for_rect(angle: f64, rect: Rect) -> crate::peniko::GradientKind {
        // The CSS spec gives this formula for the gradient line length:
        // `abs(W * sin(A)) + abs(H * cos(A))`
        // https://drafts.csswg.org/css-images-3/#linear-gradient-syntax

        let size = rect.size();
        let sin_a = angle.sin();
        let cos_a = angle.cos();
        let gradient_line_length = (size.width * sin_a).abs() + (size.height * cos_a).abs();

        let center = rect.center();
        let x = sin_a * gradient_line_length / 2.0;
        let y = cos_a * gradient_line_length / 2.0;

        let start = Point::new(center.x - x, center.y - y);
        let end = Point::new(center.x + x, center.y + y);

        crate::peniko::GradientKind::Linear { start, end }
    }
}
