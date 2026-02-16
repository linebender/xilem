// Copyright 2025 the Xilem Authors
// SPDX-License-Identifier: Apache-2.0

use crate::core::Widget;

use crate::core::{EventCtx, PointerEvent, PropertiesMut};

/// The type of a new [`Layer`].
///
/// When adding a new [layer](crate::doc::masonry_concepts#layers) to the app,
/// this tells the app driver what type of item to add.
#[derive(Clone, Debug, Default)]
pub enum LayerType {
    /// A simple tooltip showing some text until the mouse moves.
    Tooltip(String),
    /// A menu showing the different options of a selector widget.
    Selector {
        /// The text of the options.
        options: Vec<String>,
        /// The initially selected option.
        selected_option: usize,
    },
    /// Unknown layer type. Always use the widget fallback.
    #[default]
    Other,
}

/// The trait implemented by widgets which are meant to be at the root of
/// a [layer](crate::doc::masonry_concepts#layers).
pub trait Layer: Widget {
    // TODO - Possible evolutions:
    // - Return flag to suppress event from reaching children.
    // - Return flag to remove layer.
    // - Pass layer id to method.

    /// An event handler called for every layer for all pointer events, even those outside the layer's root widget.
    fn capture_pointer_event(
        &mut self,
        ctx: &mut EventCtx<'_>,
        props: &mut PropertiesMut<'_>,
        event: &PointerEvent,
    );
}
