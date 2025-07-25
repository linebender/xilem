// Copyright 2025 the Xilem Authors
// SPDX-License-Identifier: Apache-2.0

use vello::kurbo::{Point, Rect, Size, Vec2};

use crate::core::BoxConstraints;

/// An axis in visual space.
///
/// Most often used by widgets to describe
/// the direction in which they grow as their number of children increases.
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
    pub fn cross(self) -> Self {
        match self {
            Self::Horizontal => Self::Vertical,
            Self::Vertical => Self::Horizontal,
        }
    }

    /// Extract from the argument the magnitude along this axis
    pub fn major(self, size: Size) -> f64 {
        match self {
            Self::Horizontal => size.width,
            Self::Vertical => size.height,
        }
    }

    /// Extract from the argument the magnitude along the perpendicular axis
    pub fn minor(self, size: Size) -> f64 {
        self.cross().major(size)
    }

    /// Extract the extent of the argument in this axis as a pair.
    pub fn major_span(self, rect: Rect) -> (f64, f64) {
        match self {
            Self::Horizontal => (rect.x0, rect.x1),
            Self::Vertical => (rect.y0, rect.y1),
        }
    }

    /// Extract the extent of the argument in the minor axis as a pair.
    pub fn minor_span(self, rect: Rect) -> (f64, f64) {
        self.cross().major_span(rect)
    }

    /// Extract the coordinate locating the argument with respect to this axis.
    pub fn major_pos(self, pos: Point) -> f64 {
        match self {
            Self::Horizontal => pos.x,
            Self::Vertical => pos.y,
        }
    }

    /// Extract the coordinate locating the argument with respect to this axis.
    pub fn major_vec(self, vec: Vec2) -> f64 {
        match self {
            Self::Horizontal => vec.x,
            Self::Vertical => vec.y,
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
            Self::Horizontal => (major, minor),
            Self::Vertical => (minor, major),
        }
    }

    /// Generate constraints with new values on the major axis.
    pub fn constraints(self, bc: &BoxConstraints, min_major: f64, major: f64) -> BoxConstraints {
        match self {
            Self::Horizontal => BoxConstraints::new(
                Size::new(min_major, bc.min().height),
                Size::new(major, bc.max().height),
            ),
            Self::Vertical => BoxConstraints::new(
                Size::new(bc.min().width, min_major),
                Size::new(bc.max().width, major),
            ),
        }
    }
}
