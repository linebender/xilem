// Copyright 2025 the Xilem Authors
// SPDX-License-Identifier: Apache-2.0

/// Arrangement of widgets on a container's main axis.
///
/// If there is surplus space on the main axis after laying out children, this
/// enum represents how those children are positioned in this space.
#[derive(Debug, Clone, Copy, PartialEq)]
pub enum MainAxisAlignment {
    /// Widgets are packed flush to each other at the start edge of the container's main axis.
    Start,
    /// Widgets are packed flush to each other at the center of the container's main axis.
    Center,
    /// Widgets are packed flush to each other at the end edge of the container's main axis.
    End,
    /// Widgets are evenly distributed on the container's main axis.
    ///
    /// The space between each pair of widgets is the same.
    /// The first widget is flush with the start edge,
    /// and the last widget is flush with the end edge.
    SpaceBetween,
    /// Widgets are evenly distributed on the container's main axis.
    ///
    /// The space between each pair of widgets, the start edge and the first widget,
    /// and the end edge and the last widget, are all exactly the same.
    SpaceEvenly,
    /// Widgets are evenly distributed on the container's main axis.
    ///
    /// The space between each pair of widgets is the same.
    /// The space before the first and after the last widget
    /// equals half of the space between each pair of widgets.
    /// If there is only one widget, it will be centered.
    SpaceAround,
}

/// Alignment of widgets on a container's cross axis.
#[derive(Debug, Clone, Copy, PartialEq)]
pub enum CrossAxisAlignment {
    /// Widgets are placed at the start edge of the container's cross axis.
    Start,
    /// Widgets are placed at the center on the container's cross axis.
    ///
    /// If the widget's length is larger than the container's space,
    /// the widget will overflow equally in both directions.
    Center,
    /// Widgets are placed at the end edge of the container's cross axis.
    End,
    /// Widgets are placed in such a way that their first baselines align.
    ///
    /// The widget with the largest distance between its first baseline and the cross start edge
    /// will stay flush with the start edge of the container's cross axis, while
    /// all other widgets will shift towards the end edge just enough to align the first baselines.
    ///
    /// This may cause widgets to overflow the end edge.
    FirstBaseline,
    /// Widgets are placed in such a way that their last baselines align.
    ///
    /// The widget with the largest distance between its last baseline and the cross end edge
    /// will stay flush with the end edge of the container's cross axis, while
    /// all other widgets will shift towards the start edge just enough to align the last baselines.
    ///
    /// This may cause widgets to overflow the start edge.
    LastBaseline,
    /// Widgets will stretch to fill the whole container on its cross axis.
    ///
    /// Widgets that have [`Dim::Auto`] on the container's cross axis will use [`Dim::Stretch`].
    /// However, widgets with any other explicitly chosen [`Dim`] will still use that,
    /// and will be placed at the start edge of the container's cross axis.
    ///
    /// [`Dim`]: crate::layout::Dim
    /// [`Dim::Auto`]: crate::layout::Dim::Auto
    /// [`Dim::Stretch`]: crate::layout::Dim::Stretch
    Stretch,
}

impl CrossAxisAlignment {
    /// Returns the offset of this alignment given the free `space`.
    ///
    /// The free `space` is calculated by subtracting the widget's length from the container's space.
    ///
    /// This method supports negative free `space` and will return the correct negative offset.
    ///
    /// `FirstBaseline` and `LastBaseline` are fallback implementations,
    /// equivalent to `Start` and `End` respectively.
    pub fn offset(self, space: f64) -> f64 {
        match self {
            Self::Start | Self::FirstBaseline => 0.0,
            Self::Center => space / 2.0,
            Self::End | Self::LastBaseline => space,
            Self::Stretch => 0.0,
        }
    }
}
