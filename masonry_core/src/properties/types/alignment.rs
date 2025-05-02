// Copyright 2025 the Xilem Authors
// SPDX-License-Identifier: Apache-2.0

use vello::kurbo::{Point, Size};

/// Alignment describes the position of a view laid on top of another view.
///
/// See also [`VerticalAlignment`] and [`HorizontalAlignment`] for describing only a single axis.
#[derive(Default, Debug, Clone, Copy, PartialEq, Eq)]
pub enum Alignment {
    /// Align to the top left corner.
    TopLeft,
    /// Align to the center of the top edge.
    Top,
    /// Align to the top right corner.
    TopRight,
    /// Align to the center of the left edge.
    Left,
    /// Align to the center.
    #[default]
    Center,
    /// Align to the center of the right edge.
    Right,
    /// Align to the bottom left corner.
    BottomLeft,
    /// Align to the center of the bottom edge.
    Bottom,
    /// Align to the bottom right corner.
    BottomRight,
}

/// Describes the vertical position of a view laid on top of another view.
/// See also [Alignment].
#[derive(Default, Debug, Clone, Copy, PartialEq, Eq)]
pub enum VerticalAlignment {
    /// Align to the top edge.
    Top,
    /// Align to the center.
    #[default]
    Center,
    /// Align to the bottom edge.
    Bottom,
}

/// Describes the horizontal position of a view laid on top of another view.
/// See also [Alignment].
#[derive(Default, Debug, Clone, Copy, PartialEq, Eq)]
pub enum HorizontalAlignment {
    /// Align to the left edge.
    Left,
    #[default]
    /// Align to the center.
    Center,
    /// Align to the right edge.
    Right,
}

// --- MARK: IMPL ALIGNMENTS ---

impl Alignment {
    /// Constructs a new Alignment from a [vertical][VerticalAlignment] and [horizontal][HorizontalAlignment] alignment.
    pub fn new(vertical: VerticalAlignment, horizontal: HorizontalAlignment) -> Self {
        match (vertical, horizontal) {
            (VerticalAlignment::Top, HorizontalAlignment::Left) => Self::TopLeft,
            (VerticalAlignment::Top, HorizontalAlignment::Center) => Self::Top,
            (VerticalAlignment::Top, HorizontalAlignment::Right) => Self::TopRight,
            (VerticalAlignment::Center, HorizontalAlignment::Left) => Self::Left,
            (VerticalAlignment::Center, HorizontalAlignment::Center) => Self::Center,
            (VerticalAlignment::Center, HorizontalAlignment::Right) => Self::Right,
            (VerticalAlignment::Bottom, HorizontalAlignment::Left) => Self::BottomLeft,
            (VerticalAlignment::Bottom, HorizontalAlignment::Center) => Self::Bottom,
            (VerticalAlignment::Bottom, HorizontalAlignment::Right) => Self::BottomRight,
        }
    }

    /// Gets the vertical component of the alignment.
    pub fn vertical(self) -> VerticalAlignment {
        match self {
            Self::Center | Self::Left | Self::Right => VerticalAlignment::Center,
            Self::Top | Self::TopLeft | Self::TopRight => VerticalAlignment::Top,
            Self::Bottom | Self::BottomLeft | Self::BottomRight => VerticalAlignment::Bottom,
        }
    }

    /// Gets the horizontal component of the alignment.
    pub fn horizontal(self) -> HorizontalAlignment {
        match self {
            Self::Center | Self::Top | Self::Bottom => HorizontalAlignment::Center,
            Self::Left | Self::TopLeft | Self::BottomLeft => HorizontalAlignment::Left,
            Self::Right | Self::TopRight | Self::BottomRight => HorizontalAlignment::Right,
        }
    }

    /// Returns the position that would result in the `self` alignment.
    ///
    /// This gives the position a box of size `child_size` needs to have to be aligned
    /// within a box of size `parent_size`.
    pub fn resolve_pos(self, child_size: Size, parent_size: Size) -> Point {
        let diff_size = parent_size - child_size;
        let end_pos = Point::new(diff_size.width, diff_size.height);

        let center = Point::new(end_pos.x / 2., end_pos.y / 2.);

        match self {
            Self::TopLeft => Point::ZERO,
            Self::Top => Point::new(center.x, 0.),
            Self::TopRight => Point::new(end_pos.x, 0.),
            Self::Left => Point::new(0., center.y),
            Self::Center => center,
            Self::Right => Point::new(end_pos.x, center.y),
            Self::BottomLeft => Point::new(0., end_pos.y),
            Self::Bottom => Point::new(center.x, end_pos.y),
            Self::BottomRight => end_pos,
        }
    }
}

impl From<Alignment> for VerticalAlignment {
    fn from(value: Alignment) -> Self {
        value.vertical()
    }
}

impl From<Alignment> for HorizontalAlignment {
    fn from(value: Alignment) -> Self {
        value.horizontal()
    }
}

impl From<(VerticalAlignment, HorizontalAlignment)> for Alignment {
    fn from((vertical, horizontal): (VerticalAlignment, HorizontalAlignment)) -> Self {
        Self::new(vertical, horizontal)
    }
}

impl From<VerticalAlignment> for Alignment {
    fn from(vertical: VerticalAlignment) -> Self {
        Self::new(vertical, HorizontalAlignment::Center)
    }
}

impl From<HorizontalAlignment> for Alignment {
    fn from(horizontal: HorizontalAlignment) -> Self {
        Self::new(VerticalAlignment::Center, horizontal)
    }
}
