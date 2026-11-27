// Copyright 2025 the Xilem Authors
// SPDX-License-Identifier: Apache-2.0

use kurbo::{Point, Rect};

use crate::core::{EventCtx, PointerEvent, PropertiesMut, Widget};

/// Placement of an in-window subwindow in logical window coordinates.
#[derive(Clone, Copy, Debug)]
pub enum SubwindowPlacement {
    /// Place the subwindow's top-left corner at this point.
    Fixed(Point),
    /// Place the subwindow above this rectangle with a gap, flipping below or
    /// shifting as needed to fit in the window.
    ///
    /// The rectangle must be finite and is captured at creation time.
    Anchored(Rect),
    /// Place a menu flush below this rectangle, aligned with its left edge.
    /// Flip above or shift as needed to fit in the window.
    ///
    /// The rectangle must be finite and is captured at creation time.
    Dropdown(Rect),
}

impl SubwindowPlacement {
    /// Anchor a subwindow to a finite point in logical window coordinates.
    #[must_use]
    pub fn anchored_at(point: Point) -> Self {
        Self::Anchored(Rect::from_points(point, point))
    }
}

impl From<Point> for SubwindowPlacement {
    fn from(point: Point) -> Self {
        Self::Fixed(point)
    }
}

/// The type of a new [`Subwindow`].
///
/// When adding a new [subwindow](crate::doc::masonry_concepts#subwindows) to the app,
/// this tells the app driver what type of item to add.
#[derive(Clone, Debug, Default)]
pub enum SubwindowType {
    /// A simple tooltip showing some text until the mouse moves.
    Tooltip(String),
    /// A menu showing the different options of a selector widget.
    Selector {
        /// The text of the options.
        options: Vec<String>,
        /// The initially selected option.
        selected_option: usize,
    },
    /// Unknown subwindow type. Always use the widget fallback.
    #[default]
    Other,
}

/// The trait implemented by widgets which are meant to be at the root of
/// a [subwindow](crate::doc::masonry_concepts#subwindows).
pub trait Subwindow: Widget {
    // TODO - Possible evolutions:
    // - Return flag to suppress event from reaching children.
    // - Return flag to remove subwindow.
    // - Pass subwindow id to method.

    /// An event handler called for every subwindow for all pointer events, even those outside the subwindow's root widget.
    fn capture_pointer_event(
        &mut self,
        ctx: &mut EventCtx<'_>,
        props: &mut PropertiesMut<'_>,
        event: &PointerEvent,
    );
}
