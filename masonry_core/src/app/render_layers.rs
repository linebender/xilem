// Copyright 2026 the Xilem Authors
// SPDX-License-Identifier: Apache-2.0

//! Logical visual layers emitted by Masonry paint.
//!
//! These types are the paint-time/render-time view of Masonry layers.
//! They are distinct from the internal `LayerStack`, which owns persistent widget roots.

use crate::core::WidgetId;
use crate::imaging::record::Scene;
use kurbo::{Affine, Rect};

/// Stable identifier for one visual layer within a layer root.
///
/// Visual-layer ids are stable for a given `(root_id, ordinal)` pair, where `ordinal`
/// is the painter-order index of that visual layer within the same layer root.
#[derive(Clone, Copy, Debug, PartialEq, Eq, Hash)]
pub struct VisualLayerId {
    /// The root widget that owns this visual layer.
    pub root_id: WidgetId,
    /// Painter-order index of the visual layer within its owning root.
    pub ordinal: u32,
}

impl VisualLayerId {
    fn new(root_id: WidgetId, ordinal: u32) -> Self {
        Self { root_id, ordinal }
    }
}

/// Where a visual layer boundary came from in the widget model.
#[derive(Clone, Copy, Debug, PartialEq, Eq, Hash)]
pub enum VisualLayerBoundary {
    /// A top-level layer root from `LayerStack`.
    LayerRoot,
    /// An in-tree widget boundary created during paint.
    WidgetBoundary,
}

/// The content realization of a visual layer.
pub enum VisualLayerKind {
    /// Masonry-painted retained content, in the layer's local coordinate space.
    Scene(Scene),
    /// Host-owned external/native content identified by the layer root widget.
    External,
}

impl core::fmt::Debug for VisualLayerKind {
    fn fmt(&self, f: &mut core::fmt::Formatter<'_>) -> core::fmt::Result {
        match self {
            Self::Scene(_) => f.write_str("Scene(..)"),
            Self::External => f.write_str("External"),
        }
    }
}

/// A painted visual layer, ready for compositing or host realization.
///
/// Scene-backed layers contain retained `imaging` content in the layer's local coordinate space.
/// External layers preserve a host-managed layer boundary. In both cases, apply
/// [`transform`](Self::transform) to place the layer in window space.
pub struct VisualLayer {
    id: VisualLayerId,
    /// The content realization of this layer.
    pub kind: VisualLayerKind,
    /// Whether this layer came from a top-level `LayerStack` root or an in-tree paint split.
    pub boundary: VisualLayerBoundary,
    /// Axis-aligned bounds of this layer's content in layer-local coordinates.
    pub bounds: Rect,
    /// Optional clip to apply in layer-local coordinates when realizing the layer.
    pub clip: Option<Rect>,
    /// Transform from layer-local space to window space.
    pub transform: Affine,
    /// The root widget ID of this layer.
    pub root_id: WidgetId,
}

impl VisualLayer {
    /// Create a scene-backed layer.
    pub fn scene(
        scene: Scene,
        boundary: VisualLayerBoundary,
        bounds: Rect,
        clip: Option<Rect>,
        transform: Affine,
        root_id: WidgetId,
    ) -> Self {
        Self {
            id: VisualLayerId::new(root_id, 0),
            kind: VisualLayerKind::Scene(scene),
            boundary,
            bounds,
            clip,
            transform,
            root_id,
        }
    }

    /// Create an externally realized layer.
    pub fn external(
        boundary: VisualLayerBoundary,
        bounds: Rect,
        clip: Option<Rect>,
        transform: Affine,
        root_id: WidgetId,
    ) -> Self {
        Self {
            id: VisualLayerId::new(root_id, 0),
            kind: VisualLayerKind::External,
            boundary,
            bounds,
            clip,
            transform,
            root_id,
        }
    }

    /// Stable identifier for this visual layer.
    pub fn id(&self) -> VisualLayerId {
        self.id
    }

    /// Returns the axis-aligned bounds in window coordinates.
    pub fn window_bounds(&self) -> Rect {
        self.transform.transform_rect_bbox(self.bounds)
    }

    /// Returns the axis-aligned clip in window coordinates, if any.
    pub fn window_clip_bounds(&self) -> Option<Rect> {
        self.clip
            .map(|clip| self.transform.transform_rect_bbox(clip))
    }
}

/// Ordered visual layers emitted by Masonry paint.
///
/// Layers are ordered from bottom to top (painter order). The first layer is the base
/// application content. Additional layers represent tooltips, menus, isolated scene chunks,
/// and external/native layer boundaries.
pub struct VisualLayerPlan {
    /// Ordered visual layers in painter order.
    pub layers: Vec<VisualLayer>,
}

impl VisualLayerPlan {
    /// Create a visual-layer plan and assign stable visual-layer ids.
    pub fn new(mut layers: Vec<VisualLayer>) -> Self {
        assign_visual_layer_ids(&mut layers);
        Self { layers }
    }

    /// Iterate the external layers in painter order together with their external-layer index.
    pub fn external_layers(&self) -> impl Iterator<Item = (usize, &VisualLayer)> + '_ {
        self.layers
            .iter()
            .filter(|layer| matches!(layer.kind, VisualLayerKind::External))
            .enumerate()
    }

    /// Returns whether this plan contains any host-owned external layers.
    pub fn has_external_layers(&self) -> bool {
        self.layers
            .iter()
            .any(|layer| matches!(layer.kind, VisualLayerKind::External))
    }
}

fn assign_visual_layer_ids(layers: &mut [VisualLayer]) {
    let mut ordinals = std::collections::HashMap::<WidgetId, u32>::new();
    for layer in layers {
        let ordinal = ordinals.entry(layer.root_id).or_insert(0);
        layer.id = VisualLayerId::new(layer.root_id, *ordinal);
        *ordinal += 1;
    }
}
