// Copyright 2025 the Xilem Authors
// SPDX-License-Identifier: Apache-2.0

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
