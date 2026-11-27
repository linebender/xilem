// Copyright 2025 the Xilem Authors
// SPDX-License-Identifier: Apache-2.0

use kurbo::{Point, Rect};

use crate::core::{EventCtx, PointerEvent, PropertiesMut, Widget, WidgetId};

/// The preferred side of an anchor and alignment along that side.
pub use understory_anchor::Placement as AnchorPlacement;

/// The geometry a subwindow is anchored to.
#[derive(Clone, Copy, Debug)]
pub enum SubwindowAnchor {
    /// A fixed, finite rectangle in logical window coordinates.
    Rect(Rect),
    /// The current border box of a widget in an earlier subwindow in the stack.
    ///
    /// The position follows the widget as it moves. If the widget is absent
    /// or stashed, the subwindow keeps its last visible placement.
    Widget(WidgetId),
}

impl From<Rect> for SubwindowAnchor {
    fn from(rect: Rect) -> Self {
        Self::Rect(rect)
    }
}

/// Placement of an in-window subwindow in logical window coordinates.
#[derive(Clone, Copy, Debug)]
pub enum SubwindowPlacement {
    /// Place the subwindow's top-left corner at this point.
    Fixed(Point),
    /// Place the subwindow next to an anchor, flipping to the opposite side or
    /// shifting as needed to fit in the window.
    Anchored {
        /// A fixed rectangle or the widget to follow.
        anchor: SubwindowAnchor,
        /// The preferred side and alignment. The fallback preserves the alignment.
        preferred: AnchorPlacement,
        /// The finite distance away from the anchor in logical pixels.
        gap: f64,
    },
}

impl SubwindowPlacement {
    /// Create an [anchored placement](Self::Anchored) from a finite point in window coordinates.
    #[must_use]
    pub fn anchored_at(point: Point, preferred: AnchorPlacement, gap: f64) -> Self {
        Self::Anchored {
            anchor: SubwindowAnchor::Rect(Rect::from_points(point, point)),
            preferred,
            gap,
        }
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
