// Copyright 2026 the Xilem Authors
// SPDX-License-Identifier: Apache-2.0

use kurbo::Affine;

use crate::core::WidgetId;
use crate::imaging::PaintSink;
use crate::imaging::record::{Scene, replay_transformed};

/// Snapshot of Masonry's current visual layers in painter order.
///
/// This is the semantic paint output of Masonry. Current hosts still consume it through
/// compatibility helpers that flatten or reinterpret it, but the plan is the source of truth.
#[derive(Debug)]
pub struct VisualLayerPlan {
    /// Layers in painter order, back to front.
    pub layers: Vec<VisualLayer>,
}

impl VisualLayerPlan {
    /// Replay all layers into a sink in window coordinate space.
    ///
    /// This preserves current flattened rendering behavior for hosts that do not yet realize
    /// layers differently.
    pub fn replay_into<S>(&self, sink: &mut S)
    where
        S: PaintSink + ?Sized,
    {
        for layer in &self.layers {
            replay_transformed(&layer.scene, sink, layer.transform);
        }
    }

    /// The root visual layer, if one exists.
    ///
    /// In the current compatibility model, this is the first scene layer that
    /// flattened consumers treat as the base scene.
    pub fn root_layer(&self) -> Option<&VisualLayer> {
        self.layers.first()
    }

    /// All layers after the root layer.
    ///
    /// In the current compatibility model, these are replayed after the root layer
    /// in painter order.
    pub fn overlay_layers(&self) -> &[VisualLayer] {
        self.layers.get(1..).unwrap_or(&[])
    }
}

/// A single visual layer in Masonry's paint output.
///
/// The retained scene is stored in layer-local coordinates. Apply [`transform`](Self::transform)
/// to composite it into window space. The root layer uses the identity transform.
#[derive(Debug)]
pub struct VisualLayer {
    /// The retained `imaging` scene for this layer in layer-local coordinates.
    pub scene: Scene,
    /// Transform from layer-local space to window space.
    pub transform: Affine,
    /// The root widget that owns this layer.
    pub root_id: WidgetId,
}

#[cfg(test)]
mod tests {
    use super::{VisualLayer, VisualLayerPlan};
    use crate::core::WidgetId;
    use crate::imaging::Painter;
    use crate::imaging::record::{Scene, replay_transformed};
    use kurbo::{Affine, Rect};
    use peniko::Color;

    fn filled_scene(rect: Rect, color: Color) -> Scene {
        let mut scene = Scene::new();
        Painter::new(&mut scene).fill_rect(rect, color);
        scene
    }

    #[test]
    fn replay_into_replays_layers_in_window_space() {
        let root_scene = filled_scene(Rect::new(0.0, 0.0, 10.0, 10.0), Color::from_rgb8(255, 0, 0));
        let overlay_scene =
            filled_scene(Rect::new(0.0, 0.0, 4.0, 4.0), Color::from_rgb8(0, 0, 255));

        let plan = VisualLayerPlan {
            layers: vec![
                VisualLayer {
                    scene: root_scene.clone(),
                    transform: Affine::IDENTITY,
                    root_id: WidgetId::next(),
                },
                VisualLayer {
                    scene: overlay_scene.clone(),
                    transform: Affine::translate((20.0, 5.0)),
                    root_id: WidgetId::next(),
                },
            ],
        };

        let mut actual = Scene::new();
        plan.replay_into(&mut actual);

        let mut expected = Scene::new();
        replay_transformed(&root_scene, &mut expected, Affine::IDENTITY);
        replay_transformed(
            &overlay_scene,
            &mut expected,
            Affine::translate((20.0, 5.0)),
        );

        assert_eq!(actual, expected);
    }
}
