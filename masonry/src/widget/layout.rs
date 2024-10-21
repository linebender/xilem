use core::fmt;
use std::fmt::{Display, Formatter};
use vello::kurbo::common::FloatExt;
use crate::{kurbo, Point, Rect, Size, Vec2};

/// An axis in visual space.
///
/// Most often used by widgets to describe the direction in which they grow
/// as their number of children increases.
/// Has some methods for manipulating geometry with respect to the axis.
#[derive(Debug, Clone, Copy, PartialEq)]
pub enum Axis {
    /// The x axis
    Horizontal,
    /// The y axis
    Vertical,
}

impl Axis {
    /// Get the axis perpendicular to this one.
    pub fn cross(self) -> Axis {
        match self {
            Axis::Horizontal => Axis::Vertical,
            Axis::Vertical => Axis::Horizontal,
        }
    }

    /// Extract the extent of the argument in this axis as a pair.
    pub fn major_span(self, rect: Rect) -> (f64, f64) {
        match self {
            Axis::Horizontal => (rect.x0, rect.x1),
            Axis::Vertical => (rect.y0, rect.y1),
        }
    }

    /// Extract the extent of the argument in the minor axis as a pair.
    pub fn minor_span(self, rect: Rect) -> (f64, f64) {
        self.cross().major_span(rect)
    }

    /// Extract the coordinate locating the argument with respect to this axis.
    pub fn major_pos(self, pos: Point) -> f64 {
        match self {
            Axis::Horizontal => pos.x,
            Axis::Vertical => pos.y,
        }
    }

    /// Extract the coordinate locating the argument with respect to this axis.
    pub fn major_vec(self, vec: Vec2) -> f64 {
        match self {
            Axis::Horizontal => vec.x,
            Axis::Vertical => vec.y,
        }
    }

    /// Extract the coordinate locating the argument with respect to the perpendicular axis.
    pub fn minor_pos(self, pos: Point) -> f64 {
        self.cross().major_pos(pos)
    }

    /// Extract the coordinate locating the argument with respect to the perpendicular axis.
    pub fn minor_vec(self, vec: Vec2) -> f64 {
        self.cross().major_vec(vec)
    }

    // TODO - make_pos, make_size, make_rect
    /// Arrange the major and minor measurements with respect to this axis such that it forms
    /// an (x, y) pair.
    pub fn pack(self, major: f64, minor: f64) -> (f64, f64) {
        match self {
            Axis::Horizontal => (major, minor),
            Axis::Vertical => (minor, major),
        }
    }
}

// TODO: Document
#[derive(Debug, Clone, Copy, PartialEq)]
pub enum ContentFill {
    /// Minimum intrinsic size.
    /// Fit as small as possible on the specified axis. Shrink to the minimum wrappable component.
    Min,
    /// Maximum intrinsic size.
    /// Take up as much space as the content allows on the specified axis without wrapping.
    Max,
    /// Expand as desired up to the constraints, then wrap.
    Constrain,
    /// Attempt to fit the specified size exactly.
    Exact,
}

impl ContentFill {
    pub fn follow_f64_fill_rule(&self, child_value: f64, parent_value: f64) -> f64 {
        match self {
            ContentFill::Min | ContentFill::Max => { child_value }
            ContentFill::Constrain => {
                if child_value > parent_value {
                    parent_value
                } else {
                    child_value
                }
            }
            ContentFill::Exact => {
                parent_value
            }
        }
    }
}

/// A type that stores generic data along two axes: horizontal and vertical.
#[derive(Clone, Copy, Debug, PartialEq)]
pub struct BiAxial<T> {
    pub horizontal: T,
    pub vertical: T,
}

impl<T> BiAxial<T> {
    /// Constructs a size (planar with f64 type)
    #[inline]
    pub const fn new(horizontal: T, vertical: T) -> Self {
        BiAxial { horizontal, vertical }
    }

    /// Extract the value for the given axis.
    pub fn value_for_axis(self, axis: Axis) -> T {
        match axis {
            Axis::Horizontal => self.horizontal,
            Axis::Vertical => self.vertical,
        }
    }

    /// Construct a new BiAxial given a major axis and values for the major and minor axes.
    pub fn new_by_axis(major: T, minor: T, axis: Axis) -> BiAxial<T> {
        match axis {
            Axis::Horizontal => BiAxial::new(major, minor),
            Axis::Vertical => BiAxial::new(minor, major),
        }
    }

    pub fn raw(self) -> (T, T) {
        return (self.horizontal, self.vertical)
    }
}

/// The f64 implementation of BiAxis represents a size.
impl BiAxial<f64> {
    pub const ZERO: BiAxial<f64> = BiAxial::new(0.0, 0.0);
    pub const UNBOUNDED: BiAxial<f64> = BiAxial::new(f64::INFINITY, f64::INFINITY);

    /// Constructs a size (planar with f64 type)
    #[inline]
    pub fn new_size(width: f64, height: f64) -> Self {
        BiAxial { horizontal: width.expand(), vertical: height.expand() }
    }

    #[inline]
    pub const fn from_kurbo_size(size: Size) -> Self {
        BiAxial { horizontal: size.width, vertical: size.height }
    }

    /// Rounds the axes away from zero.
    pub fn expand(&self) -> Self {
        BiAxial::new_size(self.horizontal.expand(), self.vertical.expand())
    }

    /// Shrink by the given size.
    ///
    /// The given size is also [rounded away from zero],
    /// so that the layout is aligned to integers.
    ///
    /// [rounded away from zero]: Size::expand
    pub fn shrink(&self, diff: impl Into<BiAxial<f64>>) -> BiAxial<f64> {
        let diff = diff.into().expand();
        BiAxial::new_size(
            (self.horizontal - diff.horizontal).max(0.),
            (self.vertical - diff.vertical).max(0.),
        )
    }

    /// Return the minimum on each axis between the inputted size and the called struct.
    ///
    /// The given input is also [rounded away from zero],
    /// so that the layout is aligned to integers.
    ///
    /// [rounded away from zero]: Size::expand
    pub fn constrain(&self, other: impl Into<BiAxial<f64>>) -> Self {
        let other = other.into();
        let other_expanded = other.expand();

        let horizontal = self.horizontal.min(other_expanded.horizontal);
        let vertical = self.vertical.min(other_expanded.vertical);
        BiAxial { horizontal, vertical }
    }

    // TODO: Is this the best way to do it, or should we potentially merge this with constrain,
    //    or have self be the parent, and the child size passed in as a param?
    pub fn use_fill_mode(&self, axis_rules: &BiAxial<ContentFill>, parent_size: &BiAxial<f64>) -> BiAxial<f64> {
        BiAxial {
            horizontal: axis_rules.horizontal.follow_f64_fill_rule(self.horizontal, parent_size.horizontal),
            vertical: axis_rules.vertical.follow_f64_fill_rule(self.vertical, parent_size.vertical),
        }
    }

    pub fn constrain_and_fill(&self, axis_rules: &BiAxial<ContentFill>, other: impl Into<BiAxial<f64>>) -> BiAxial<f64> {
        self.constrain(other).use_fill_mode(axis_rules, self)
    }

    // TODO: Documentation from BC
    pub fn constrain_aspect_ratio(&self, aspect_ratio: f64, width: f64) -> BiAxial<f64> {
        // Minimizing/maximizing based on aspect ratio seems complicated, but in reality everything
        // is linear, so the amount of work to do is low.
        let ideal_size = BiAxial::new_size(width, width * aspect_ratio);

        // It may be possible to remove these in the future if the invariant is checked elsewhere.
        let aspect_ratio = aspect_ratio.abs();
        let width = width.abs();

        // Firstly check if we can simply return the exact requested
        if self.contains(ideal_size) {
            return ideal_size;
        }

        // Then we check if any `Size`s with our desired aspect ratio are inside the constraints.
        // TODO this currently outputs garbage when things are < 0 - See https://github.com/linebender/xilem/issues/377
        let min_w_min_h = 0.0 / 0.0;
        let max_w_min_h = 0.0 / self.horizontal;
        let min_w_max_h = self.vertical / 0.0;
        let max_w_max_h = self.vertical / self.horizontal;

        // When the aspect ratio line crosses the constraints, the closest point must be one of the
        // two points where the aspect ratio enters/exits.

        // When the aspect ratio line doesn't intersect the box of possible sizes, the closest
        // point must be either (max width, min height) or (max height, min width). So all we have
        // to do is check which one of these has the closest aspect ratio.

        // Check each possible intersection (or not) of the aspect ratio line with the constraints
        if aspect_ratio > min_w_max_h {
            // outside max height min width
            BiAxial::new_size(0.0, self.vertical)
        } else if aspect_ratio < max_w_min_h {
            // outside min height max width
            BiAxial::new_size(self.horizontal, 0.0)
        } else if aspect_ratio > min_w_min_h {
            // hits the constraints on the min width line
            if width < 0.0 {
                // we take the point on the min width
                BiAxial::new_size(0.0, 0.0 * aspect_ratio)
            } else if aspect_ratio < max_w_max_h {
                // exits through max.width
                BiAxial::new_size(self.horizontal, self.horizontal * aspect_ratio)
            } else {
                // exits through max.height
                BiAxial::new_size(self.vertical * aspect_ratio.recip(), self.vertical)
            }
        } else {
            // final case is where we hit constraints on the min height line
            if width < 0.0 {
                // take the point on the min height
                BiAxial::new_size(0.0 * aspect_ratio.recip(), 0.0)
            } else if aspect_ratio > max_w_max_h {
                // exit thru max height
                BiAxial::new_size(self.vertical * aspect_ratio.recip(), self.vertical)
            } else {
                // exit thru max width
                BiAxial::new_size(self.horizontal, self.horizontal * aspect_ratio)
            }
        }
    }

    pub fn contains(&self, size: impl Into<BiAxial<f64>>) -> bool {
        let size = size.into();
        (size.horizontal <= self.horizontal) && (size.vertical <= self.vertical)
    }

    pub fn to_size(&self) -> Size {
        return Size::new(self.horizontal, self.vertical)
    }

    pub fn is_zero_area(&self) -> bool {
        return self.horizontal * self.vertical <= 0.0;
    }

    pub fn has_infinite_value(&self) -> bool {
        return self.horizontal.is_infinite() || self.vertical.is_infinite()
    }

    /// Whether there is an upper bound on the width.
    pub fn is_width_bounded(&self) -> bool {
        self.horizontal.is_finite()
    }

    /// Whether there is an upper bound on the height.
    pub fn is_height_bounded(&self) -> bool {
        self.vertical.is_finite()
    }

    pub fn validate_sizes(&self, name: &str) {
        if cfg!(not(debug_assertions)) {
            return;
        }

        if !(0.0 <= self.horizontal && 0.0 <= self.vertical && self.horizontal.expand() == self.horizontal) {
            tracing::warn!("Bad Planar value passed to {}:", name);
            tracing::warn!("{:?}", self);
        }

        if self.horizontal.is_nan() {
            debug_panic!("Horizontal value in Planar passed to {name} is NaN");
        }
        if self.vertical.is_nan() {
            debug_panic!("Vertical value in Planar passed to {name} is NaN");
        }
    }
}

impl From<(f64, f64)> for BiAxial<f64> {
    #[inline]
    fn from(v: (f64, f64)) -> BiAxial<f64> {
        BiAxial::new_size(v.0, v.1)
    }
}

impl From<BiAxial<f64>> for (f64, f64) {
    #[inline]
    fn from(v: BiAxial<f64>) -> (f64, f64) {
        (v.horizontal, v.vertical)
    }
}

impl Display for BiAxial<f64> {
    fn fmt(&self, f: &mut fmt::Formatter) -> fmt::Result {
        write!(f, "{:?}W×{:?}H", self.horizontal, self.vertical)
    }
}

// TODO: Test expand
// TODO: Test constrain