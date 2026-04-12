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

/// How a layer root wants its content to be realized by the host.
#[derive(Clone, Copy, Debug, Default, PartialEq, Eq)]
pub enum LayerRealization {
    /// Masonry paints this layer into a retained `imaging` scene.
    #[default]
    Scene,
    /// Masonry preserves this layer boundary for host-managed realization.
    ///
    /// This is intended for content such as foreign surfaces, 3D viewports, or
    /// platform-native compositor layers.
    External,
}

/// How a widget subtree participates in the ordered paint layer plan.
#[derive(Clone, Copy, Debug, Default, PartialEq, Eq)]
pub enum PaintLayerMode {
    /// Paint inline into the parent scene chunk.
    #[default]
    Inline,
    /// Paint the subtree into its own retained scene layer while preserving painter order.
    IsolatedScene,
    /// Reserve the subtree as a host-managed external layer.
    ///
    /// Masonry still traverses the subtree to clear paint invalidation flags, but its retained
    /// scene output is discarded and the host is expected to realize the layer separately.
    External,
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

    /// Returns how this layer wants its contents to be realized.
    ///
    /// The default is [`LayerRealization::Scene`], which means Masonry paints the layer into
    /// a retained `imaging` scene. Layers that represent foreign or host-native content can
    /// override this to [`LayerRealization::External`] so render backends preserve the layer
    /// boundary instead of flattening it.
    fn realization(&self) -> LayerRealization {
        LayerRealization::Scene
    }
}
