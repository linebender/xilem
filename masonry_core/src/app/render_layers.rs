// Copyright 2026 the Xilem Authors
// SPDX-License-Identifier: Apache-2.0

//! Logical visual layers emitted by Masonry paint.
//!
//! These types are the paint-time/render-time view of Masonry layers.
//! They are distinct from the internal `LayerStack`, which owns persistent widget roots.

use crate::core::WidgetId;
use crate::imaging::PaintSink;
use crate::imaging::record::{Scene, replay_transformed};
use kurbo::{Affine, Rect};

/// The kind of host-owned external layer preserved in the visual layer plan.
#[derive(Clone, Copy, Debug, PartialEq, Eq, Hash)]
pub enum ExternalLayerKind {
    /// A host-managed surface slot reserved within the widget tree.
    Surface,
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
    External(ExternalLayerKind),
}

impl core::fmt::Debug for VisualLayerKind {
    fn fmt(&self, f: &mut core::fmt::Formatter<'_>) -> core::fmt::Result {
        match self {
            Self::Scene(_) => f.write_str("Scene(..)"),
            Self::External(kind) => f.debug_tuple("External").field(kind).finish(),
        }
    }
}

/// A painted visual layer, ready for compositing or host realization.
///
/// Scene-backed layers contain retained `imaging` content in the layer's local coordinate space.
/// External layers preserve a host-managed layer boundary. In both cases, apply
/// [`transform`](Self::transform) to place the layer in window space.
pub struct VisualLayer {
    /// The content realization of this layer.
    pub kind: VisualLayerKind,
    /// Where this visual layer boundary originated.
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
        kind: ExternalLayerKind,
        boundary: VisualLayerBoundary,
        bounds: Rect,
        clip: Option<Rect>,
        transform: Affine,
        root_id: WidgetId,
    ) -> Self {
        Self {
            kind: VisualLayerKind::External(kind),
            boundary,
            bounds,
            clip,
            transform,
            root_id,
        }
    }

    /// Returns the external-layer kind, if this is host-owned content.
    pub fn external_kind(&self) -> Option<ExternalLayerKind> {
        match self.kind {
            VisualLayerKind::External(kind) => Some(kind),
            VisualLayerKind::Scene(_) => None,
        }
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
    /// Replay all scene-backed layers into a sink in window coordinate space.
    ///
    /// This is the backend-agnostic way to consume Masonry's retained paint output.
    pub fn replay_into<S>(&self, sink: &mut S)
    where
        S: PaintSink + ?Sized,
    {
        for layer in &self.layers {
            if let VisualLayerKind::Scene(scene) = &layer.kind {
                replay_transformed(scene, sink, layer.transform);
            }
        }
    }

    /// Iterate the external layers in painter order together with their external-layer index.
    pub fn external_layers(&self) -> impl Iterator<Item = (usize, &VisualLayer)> {
        self.layers
            .iter()
            .filter(|layer| layer.external_kind().is_some())
            .enumerate()
    }

    /// Returns whether this plan contains any host-owned external layers.
    pub fn has_external_layers(&self) -> bool {
        self.layers
            .iter()
            .any(|layer| layer.external_kind().is_some())
    }
}
