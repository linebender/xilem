// Copyright 2025 the Xilem Authors
// SPDX-License-Identifier: Apache-2.0

use vello::kurbo::{Point, Rect};

use crate::peniko::color::{ColorSpaceTag, HueDirection};
use crate::peniko::{ColorStops, ColorStopsSource, Extend};
use crate::properties::types::UnitPoint;

/// Properties for the supported [`Gradient`] types.
///
/// This mirrors [`peniko::GradientKind`](crate::peniko::GradientKind),
/// but uses a layout-invariant representation: instead of saying
/// "The gradient goes from point A to point B", we declare things like
/// "The gradient has angle X", and A and B are computed dynamically from widget layout.
#[derive(Clone, Debug, PartialEq)]
#[non_exhaustive]
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
    /// Gradient that transitions between two or more colors that radiate from an origin.
    ///
    /// This is interpreted like [`radial-gradient()`] in CSS.
    ///
    /// [`radial-gradient()`]: https://drafts.csswg.org/css-images-3/#radial-gradient-syntax
    Radial {
        /// The center of the gradient, relative to the widget's bounding box.
        center: UnitPoint,
        /// The shape and size of the gradient.
        shape: RadialGradientShape,
    },
    // TODO - Add Sweep shape
}

/// Definition of a gradient that transitions between two or more colors.
///
/// This mirrors [`peniko::Gradient`](crate::peniko::Gradient),
/// but uses a layout-invariant representation: instead of saying
/// "The gradient goes from point A to point B", we declare things like
/// "The gradient has angle X", and A and B and computed dynamically from widget layout.
#[derive(Clone, Debug, PartialEq)]
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

/// An enum for different ways a radial gradient can be sized and shaped.
///
/// Matches the `radial-shape` and `radial-extent` parameters
/// of the [`radial-gradient()`] syntax.
///
/// Defaults to `CircleTo(FarthestCorner)`.
///
/// [`radial-gradient()`]: https://drafts.csswg.org/css-images-3/#radial-gradient-syntax
#[non_exhaustive]
#[derive(Clone, Copy, Debug, PartialEq)]
pub enum RadialGradientShape {
    /// A circle defined based on the box size.
    CircleTo(RadialGradientExtent),
    /// A circle with a fixed radius.
    FixedCircle(f64),
    // TODO - Add following and remove #[non_exhaustive]:
    // EllipseTo(RadialGradientExtent),
    // FixedEllipse(f64),
    // EllipsePercentage(f64),
}

/// An enum for different ways a radial gradient can be sized based on the surrounding box.
///
/// Matches the `radial-extent` parameter of the [`radial-gradient()`] syntax.
///
/// [`radial-gradient()`]: https://drafts.csswg.org/css-images-3/#radial-gradient-syntax
#[derive(Clone, Copy, Debug, PartialEq)]
pub enum RadialGradientExtent {
    /// Interpreted like CSS `closest-corner` size.
    ClosestCorner,
    /// Interpreted like CSS `closest-side` size.
    ClosestSide,
    /// Interpreted like CSS `farthest-side` size.
    FarthestSide,
    /// Interpreted like CSS `farthest-corner` size.
    FarthestCorner,
}

// ---

impl Gradient {
    /// Creates a gradient with the given shape.
    ///
    /// See also [`Self::new_linear`], [`Self::new_radial`], [`Self::new_radial_with`].
    pub fn new(shape: GradientShape) -> Self {
        Self {
            shape,
            extend: Extend::default(),
            interpolation_cs: ColorSpaceTag::Srgb,
            hue_direction: HueDirection::default(),
            stops: ColorStops::default(),
        }
    }

    /// Creates a [`Linear`](GradientShape::Linear) gradient.
    ///
    /// `angle` is in radians, with zero pointing upwards, and higher values rotating the gradient clockwise.
    /// This matches how [CSS gradients are defined](https://developer.mozilla.org/en-US/docs/Web/CSS/gradient/linear-gradient#angle).
    ///
    /// That is, for an `angle` of 0, the first stop will be at the bottom, and
    /// for an `angle` of [`π/2`](core::f32::consts::FRAC_PI_2) (90°), the first
    /// stop will be aligned with the left edge.
    pub fn new_linear(angle: f64) -> Self {
        let shape = GradientShape::Linear { angle };
        Self::new(shape)
    }

    /// Creates a circular [`Radial`](GradientShape::Radial) gradient extending to the farthest corner.
    ///
    /// See also [`Self::new_radial_with`].
    pub fn new_radial(center: UnitPoint) -> Self {
        let shape = RadialGradientShape::CircleTo(RadialGradientExtent::FarthestCorner);
        let shape = GradientShape::Radial { center, shape };
        Self::new(shape)
    }

    /// Creates a [`Radial`](GradientShape::Radial) gradient.
    pub fn new_radial_with(center: UnitPoint, shape: RadialGradientShape) -> Self {
        let shape = GradientShape::Radial { center, shape };
        Self::new(shape)
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
            Self::Radial { center, shape } => {
                Self::get_peniko_radial_for_rect(*center, *shape, rect)
            }
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

    #[expect(
        clippy::cast_possible_truncation,
        reason = "no other way to go from f64 to f32"
    )]
    fn get_peniko_radial_for_rect(
        center: UnitPoint,
        shape: RadialGradientShape,
        rect: Rect,
    ) -> crate::peniko::GradientKind {
        let center = center.resolve(rect);
        let radius = Self::get_gradient_radius(rect, center, shape);

        crate::peniko::GradientKind::Radial {
            start_center: center,
            start_radius: 0.,
            end_center: center,
            end_radius: radius as f32,
        }
    }

    fn get_gradient_radius(rect: Rect, center: Point, shape: RadialGradientShape) -> f64 {
        let center_offset = center - rect.origin();

        let dist_to_sides = [
            f64::abs(center_offset.x),
            f64::abs(center_offset.y),
            f64::abs(rect.size().width - center_offset.x),
            f64::abs(rect.size().height - center_offset.y),
        ];
        let dist_to_corners = [
            center.distance(Point::new(rect.x0, rect.y0)),
            center.distance(Point::new(rect.x1, rect.y0)),
            center.distance(Point::new(rect.x0, rect.y1)),
            center.distance(Point::new(rect.x1, rect.y1)),
        ];

        match shape {
            RadialGradientShape::CircleTo(RadialGradientExtent::ClosestCorner) => {
                dist_to_corners.into_iter().reduce(f64::min).unwrap()
            }
            RadialGradientShape::CircleTo(RadialGradientExtent::ClosestSide) => {
                dist_to_sides.into_iter().reduce(f64::min).unwrap()
            }
            RadialGradientShape::CircleTo(RadialGradientExtent::FarthestSide) => {
                dist_to_sides.into_iter().reduce(f64::max).unwrap()
            }
            RadialGradientShape::CircleTo(RadialGradientExtent::FarthestCorner) => {
                dist_to_corners.into_iter().reduce(f64::max).unwrap()
            }
            RadialGradientShape::FixedCircle(radius) => radius,
        }
    }
}

impl Default for RadialGradientShape {
    fn default() -> Self {
        Self::CircleTo(RadialGradientExtent::FarthestCorner)
    }
}
