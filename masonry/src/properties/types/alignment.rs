// Copyright 2025 the Xilem Authors
// SPDX-License-Identifier: Apache-2.0

/// The alignment of the widgets on a container's cross (or minor) axis.
///
/// If a widget is smaller than the container on the minor axis, this determines
/// where it is positioned.
#[derive(Debug, Clone, Copy, PartialEq)]
pub enum CrossAxisAlignment {
    /// Top or left.
    Start,
    /// Widgets are centered in the container.
    Center,
    /// Bottom or right.
    End,
    /// Align on the baseline.
    Baseline,
    /// Fill the available space.
    Fill,
}

/// Arrangement of children on a container's main axis.
///
/// If there is surplus space on the main axis after laying out children, this
/// enum represents how children are laid out in this space.
#[derive(Debug, Clone, Copy, PartialEq)]
pub enum MainAxisAlignment {
    /// Top or left.
    Start,
    /// Children are centered, without padding.
    Center,
    /// Bottom or right.
    End,
    /// Extra space is divided evenly between each child.
    SpaceBetween,
    /// Extra space is divided evenly between each child, as well as at the ends.
    SpaceEvenly,
    /// Space between each child, with less at the start and end.
    SpaceAround,
}

impl CrossAxisAlignment {
    /// Given the difference between the size of the container and the size
    /// of the child (on their minor axis) return the necessary offset for
    /// this alignment.
    pub fn align(self, val: f64) -> f64 {
        match self {
            Self::Start => 0.0,
            // in vertical layout, baseline is equivalent to center
            Self::Center | Self::Baseline => (val / 2.0).round(),
            Self::End => val,
            Self::Fill => 0.0,
        }
    }
}
