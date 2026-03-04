// Copyright 2024 the Xilem Authors
// SPDX-License-Identifier: Apache-2.0

use std::collections::HashMap;

use tracing::{info_span, trace};
use tree_arena::ArenaMut;
use vello::Scene;
use vello::kurbo::{Affine, Line, Stroke};
use vello::peniko::{Color, Fill};

use crate::app::{RenderRoot, RenderRootState};
use crate::core::{DefaultProperties, PaintCtx, PropertiesRef, WidgetArenaNode, WidgetId};
use crate::passes::{enter_span_if, recurse_on_children};
use crate::util::{get_debug_color, stroke};

/// A painted overlay layer, ready for compositing.
///
/// The scene is in the layer's local coordinate space. To composite into
/// window space, apply the layer's [`transform`](Self::transform).
pub struct PaintedLayer {
    /// The rendered scene for this layer, in the layer's local coordinate space.
    pub scene: Scene,
    /// Transform from layer-local space to window space.
    ///
    /// For layers placed at a simple position, this is a translation.
    /// Apply this transform when compositing the layer's scene into window space.
    pub transform: Affine,
    /// The root widget ID of this layer.
    pub root_id: WidgetId,
}

/// Result of the paint pass — one scene per layer.
///
/// The base layer contains the main application content in window coordinate space.
/// Overlay layers contain tooltips, menus, and other popups in layer-local coordinate
/// space, ordered from bottom to top (painter's order).
pub struct PaintResult {
    /// The base layer scene (main application content) in window coordinate space.
    pub base: Scene,
    /// Overlay layer scenes in z-order (bottom to top), each in layer-local coordinates.
    pub overlays: Vec<PaintedLayer>,
}

impl PaintResult {
    /// Recomposite all layers into a single scene in window coordinate space.
    pub fn composite(&self) -> Scene {
        let mut scene = self.base.clone();
        for layer in &self.overlays {
            scene.append(&layer.scene, Some(layer.transform));
        }
        scene
    }
}

// --- MARK: PAINT WIDGET
fn paint_widget(
    global_state: &mut RenderRootState,
    default_properties: &DefaultProperties,
    complete_scene: &mut Scene,
    scene_cache: &mut HashMap<WidgetId, (Scene, Scene, Scene)>,
    window_to_layer_transform: &Affine,
    node: ArenaMut<'_, WidgetArenaNode>,
) {
    let mut children = node.children;
    let widget = &mut *node.item.widget;
    let state = &mut node.item.state;
    let properties = &mut node.item.properties;
    let id = state.id;

    let trace = global_state.trace.paint;
    let _span = enter_span_if(trace, state);

    // Note: At this point we could short-circuit if is_stashed is true,
    // but we deliberately avoid doing that to avoid creating zombie flags.
    // (See WidgetState doc.)
    let is_stashed = state.is_stashed;

    // TODO - Handle damage regions
    // https://github.com/linebender/xilem/issues/789

    if (state.request_pre_paint || state.request_paint || state.request_post_paint) && !is_stashed {
        if trace {
            trace!("Painting widget '{}' {}", widget.short_type_name(), id);
        }

        let mut ctx = PaintCtx {
            global_state,
            widget_state: state,
            children: children.reborrow_mut(),
        };
        let props = PropertiesRef {
            set: properties,
            default_map: default_properties.for_widget(widget.type_id()),
        };

        // TODO - Reserve scene
        // https://github.com/linebender/xilem/issues/524
        let (pre_scene, scene, post_scene) = scene_cache.entry(id).or_default();

        if state.request_pre_paint {
            pre_scene.reset();
            widget.pre_paint(&mut ctx, &props, pre_scene);
        }
        if state.request_paint {
            scene.reset();
            widget.paint(&mut ctx, &props, scene);
        }
        if state.request_post_paint {
            post_scene.reset();
            widget.post_paint(&mut ctx, &props, post_scene);
        }
    }

    state.request_pre_paint = false;
    state.request_paint = false;
    state.request_post_paint = false;
    state.needs_paint = false;

    let border_box_to_layer_transform = *window_to_layer_transform * state.window_transform;
    let content_box_to_layer_transform =
        border_box_to_layer_transform.pre_translate(state.border_box_translation());
    let has_clip = state.clip_path.is_some();
    if !is_stashed {
        let Some((pre_scene, scene, _)) = &mut scene_cache.get(&id) else {
            debug_panic!(
                "Error in paint pass: scene should have been cached earlier in this function."
            );
            return;
        };

        complete_scene.append(pre_scene, Some(content_box_to_layer_transform));

        if let Some(clip) = state.clip_path {
            // The clip path is stored in border-box space, so need to use that transform.
            complete_scene.push_clip_layer(Fill::NonZero, border_box_to_layer_transform, &clip);
        }

        complete_scene.append(scene, Some(content_box_to_layer_transform));
    }

    let parent_state = &mut *state;
    recurse_on_children(id, widget, children, |mut node| {
        // TODO: We could skip painting children outside the parent clip path.
        // There's a few things to consider if we do:
        // - Some widgets can paint outside of their layout box.
        // - Once we implement compositor layers, we may want to paint outside of the clip path anyway in anticipation of user scrolling.
        // - We still want to reset needs_paint and request_paint flags.
        paint_widget(
            global_state,
            default_properties,
            complete_scene,
            scene_cache,
            window_to_layer_transform,
            node.reborrow_mut(),
        );
        parent_state.merge_up(&mut node.item.state);
    });

    if !is_stashed {
        if global_state.debug_paint {
            // Draw the global axis aligned bounding rect of the widget
            const BORDER_WIDTH: f64 = 1.0;
            let color = get_debug_color(id.to_raw());
            let rect = state.bounding_box.inset(BORDER_WIDTH / -2.0);
            stroke(complete_scene, &rect, color, BORDER_WIDTH);

            // Draw the widget's explicit baselines
            let mut draw_baseline = |baseline| {
                let line = Line::new((0., baseline), (state.end_point.x, baseline));
                let baseline_style = Stroke::new(1.0).with_dashes(0., [4.0, 4.0]);
                complete_scene.stroke(
                    &baseline_style,
                    border_box_to_layer_transform,
                    color,
                    None,
                    &line,
                );
            };
            if !state.first_baseline.is_nan() {
                draw_baseline(state.first_baseline);
            }
            if !state.last_baseline.is_nan() && state.last_baseline != state.first_baseline {
                draw_baseline(state.last_baseline);
            }
        }

        if has_clip {
            complete_scene.pop_layer();
        }

        let Some((_, _, post_scene)) = &mut scene_cache.get(&id) else {
            debug_panic!(
                "Error in paint pass: scene should have been cached earlier in this function."
            );
            return;
        };

        complete_scene.append(post_scene, Some(content_box_to_layer_transform));
    }
}

// --- MARK: ROOT
/// See the [passes documentation](crate::doc::pass_system#render-passes).
pub(crate) fn run_paint_pass(root: &mut RenderRoot) -> PaintResult {
    let _span = info_span!("paint").entered();

    // TODO - This is a bit of a hack until we refactor widget tree mutation.
    // This should be removed once remove_child is exclusive to MutateCtx.
    let mut scene_cache = std::mem::take(&mut root.global_state.scene_cache);

    let root_id = root.root_id();
    let layer_ids = root.layer_ids();

    // Paint each layer into its own scene.
    let mut base_scene = Scene::new();
    let mut overlays = Vec::new();

    for (idx, &layer_widget_id) in layer_ids.iter().enumerate() {
        if idx == 0 {
            paint_layer(
                root,
                &mut base_scene,
                &mut scene_cache,
                root_id,
                layer_widget_id,
            );
        } else {
            let mut layer_scene = Scene::new();
            paint_layer(
                root,
                &mut layer_scene,
                &mut scene_cache,
                root_id,
                layer_widget_id,
            );

            let layer_to_window_transform = root
                .widget_arena
                .get_state(layer_widget_id)
                .window_transform;

            overlays.push(PaintedLayer {
                scene: layer_scene,
                transform: layer_to_window_transform,
                root_id: layer_widget_id,
            });
        }
    }

    root.global_state.scene_cache = scene_cache;

    // Display a rectangle over the hovered widget, in the layer that owns it.
    if let Some(hovered_widget) = root.global_state.inspector_state.hovered_widget {
        const HOVER_FILL_COLOR: Color = Color::from_rgba8(60, 60, 250, 100);
        let state = root.widget_arena.get_state(hovered_widget);
        let rect = state.border_box_size().to_rect();
        let border_box_to_window_transform = state.window_transform;

        // Walk up the widget tree to find which layer root this widget belongs to.
        let mut layer_root = hovered_widget;
        while let Some(parent) = root.widget_arena.parent_of(layer_root) {
            if parent == root_id {
                break;
            }
            layer_root = parent;
        }

        let window_to_layer_transform = root
            .widget_arena
            .get_state(layer_root)
            .window_transform
            .inverse();
        let border_box_to_layer_transform =
            window_to_layer_transform * border_box_to_window_transform;

        // Draw the hover rect in the owning layer's scene.
        if layer_root == layer_ids[0] {
            base_scene.fill(
                Fill::NonZero,
                border_box_to_layer_transform,
                HOVER_FILL_COLOR,
                None,
                &rect,
            );
        } else if let Some(layer) = overlays.iter_mut().find(|l| l.root_id == layer_root) {
            layer.scene.fill(
                Fill::NonZero,
                border_box_to_layer_transform,
                HOVER_FILL_COLOR,
                None,
                &rect,
            );
        } else {
            // Fallback: draw in base scene.
            base_scene.fill(
                Fill::NonZero,
                border_box_to_layer_transform,
                HOVER_FILL_COLOR,
                None,
                &rect,
            );
        }
    }

    PaintResult {
        base: base_scene,
        overlays,
    }
}

/// Paint a single layer's widget subtree into `target_scene`.
///
/// This is a helper that handles the split borrows needed to access
/// `global_state`, `default_properties`, and `widget_arena` simultaneously.
fn paint_layer(
    root: &mut RenderRoot,
    target_scene: &mut Scene,
    scene_cache: &mut HashMap<WidgetId, (Scene, Scene, Scene)>,
    root_id: WidgetId,
    layer_widget_id: WidgetId,
) {
    // Clear the LayerStack's own paint flags (its paint is a no-op).
    // This is idempotent so safe to call per-layer.
    {
        let root_node = root.widget_arena.get_node_mut(root_id);
        let state = &mut root_node.item.state;
        state.request_pre_paint = false;
        state.request_paint = false;
        state.request_post_paint = false;
        state.needs_paint = false;
    }

    // Get the layer child from the arena, then pass split borrows to paint_widget.
    let root_node = root.widget_arena.get_node_mut(root_id);
    let Some(layer_node) = root_node.children.into_item_mut(layer_widget_id) else {
        debug_panic!(
            "Error in paint pass: cannot find layer child {layer_widget_id:?} in LayerStack"
        );
        return;
    };

    let window_to_layer_transform = layer_node.item.state.window_transform.inverse();

    paint_widget(
        &mut root.global_state,
        &root.default_properties,
        target_scene,
        scene_cache,
        &window_to_layer_transform,
        layer_node,
    );
}
