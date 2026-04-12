// Copyright 2024 the Xilem Authors
// SPDX-License-Identifier: Apache-2.0

use std::collections::HashMap;

use kurbo::{Affine, Line, Rect, Stroke};
use peniko::{Color, Fill};
use tracing::{info_span, trace};
use tree_arena::ArenaMut;

use crate::app::{
    ExternalLayerKind, RenderRoot, RenderRootState, VisualLayer, VisualLayerBoundary,
    VisualLayerKind, VisualLayerPlan,
};
use crate::core::{
    DefaultProperties, LayerRealization, PaintCtx, PaintLayerMode, PropertiesRef, PropertyArena,
    WidgetArenaNode, WidgetId,
};
use crate::imaging::record::{Clip, Geometry, Scene};
use crate::imaging::{PaintSink, Painter};
use crate::passes::{enter_span_if, recurse_on_children};
use crate::util::get_debug_color;

fn flush_scene_layer(
    scene: &mut Scene,
    layers: &mut Vec<VisualLayer>,
    boundary: VisualLayerBoundary,
    bounds: Rect,
    clip: Option<Rect>,
    transform: Affine,
    root_id: WidgetId,
) {
    if !scene.commands().is_empty() {
        layers.push(VisualLayer::scene(
            std::mem::take(scene),
            boundary,
            bounds,
            clip,
            transform,
            root_id,
        ));
    }
}

#[derive(Clone, Copy)]
struct LayerPaintState {
    root_id: WidgetId,
    boundary: VisualLayerBoundary,
    bounds: Rect,
    clip: Option<Rect>,
    transform: Affine,
    window_to_layer_transform: Affine,
}

fn paint_subtree_as_layers(
    global_state: &mut RenderRootState,
    default_properties: &DefaultProperties,
    property_arena: &PropertyArena,
    scene_cache: &mut HashMap<WidgetId, (Scene, Scene, Scene)>,
    layer: LayerPaintState,
    node: ArenaMut<'_, WidgetArenaNode>,
) -> Vec<VisualLayer> {
    let mut current_scene = Scene::new();
    let mut layers = Vec::new();
    paint_widget(
        global_state,
        default_properties,
        property_arena,
        &mut current_scene,
        &mut layers,
        scene_cache,
        layer,
        node,
    );
    flush_scene_layer(
        &mut current_scene,
        &mut layers,
        layer.boundary,
        layer.bounds,
        layer.clip,
        layer.transform,
        layer.root_id,
    );
    layers
}

// --- MARK: PAINT WIDGET
fn paint_widget(
    global_state: &mut RenderRootState,
    default_properties: &DefaultProperties,
    property_arena: &PropertyArena,
    current_scene: &mut Scene,
    layers: &mut Vec<VisualLayer>,
    scene_cache: &mut HashMap<WidgetId, (Scene, Scene, Scene)>,
    current_layer: LayerPaintState,
    node: ArenaMut<'_, WidgetArenaNode>,
) {
    let mut children = node.children;
    let widget = &mut *node.item.widget;
    let state = &mut node.item.state;
    let properties = &mut node.item.properties;
    let class_set = &node.item.class_set;
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

        let stack = property_arena.get(state.property_stack_id, widget.type_id());
        let mut ctx = PaintCtx {
            global_state,
            widget_state: state,
            children: children.reborrow_mut(),
        };
        let props = PropertiesRef {
            local: properties,
            default_map: default_properties.for_widget(widget.type_id()),
            stack,
            class_set,
        };

        // TODO - Reserve scene
        // https://github.com/linebender/xilem/issues/524
        let (pre_scene, scene, post_scene) = scene_cache.entry(id).or_default();
        if ctx.widget_state.request_pre_paint {
            pre_scene.clear();
            let sink_dyn: &mut dyn PaintSink = pre_scene;
            let mut painter = Painter::new(sink_dyn);
            widget.pre_paint(&mut ctx, &props, &mut painter);
        }
        if ctx.widget_state.request_paint {
            scene.clear();
            let sink_dyn: &mut dyn PaintSink = scene;
            let mut painter = Painter::new(sink_dyn);
            widget.paint(&mut ctx, &props, &mut painter);
        }
        if ctx.widget_state.request_post_paint {
            post_scene.clear();
            let sink_dyn: &mut dyn PaintSink = post_scene;
            let mut painter = Painter::new(sink_dyn);
            widget.post_paint(&mut ctx, &props, &mut painter);
        }
    }

    state.request_pre_paint = false;
    state.request_paint = false;
    state.request_post_paint = false;
    state.needs_paint = false;

    let border_box_to_layer_transform =
        current_layer.window_to_layer_transform * state.window_transform;
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

        current_scene.append_transformed(pre_scene, content_box_to_layer_transform);

        if let Some(clip) = state.clip_path {
            // The clip path is stored in border-box space, so need to use that transform.
            current_scene.push_clip(Clip::Fill {
                transform: border_box_to_layer_transform,
                shape: Geometry::Rect(clip),
                fill_rule: Fill::NonZero,
            });
        }

        current_scene.append_transformed(scene, content_box_to_layer_transform);
    }

    let parent_state = &mut *state;
    recurse_on_children(id, widget, children, |mut node| {
        // TODO: We could skip painting children outside the parent clip path.
        // There's a few things to consider if we do:
        // - Some widgets can paint outside of their layout box.
        // - Once we implement compositor layers, we may want to paint outside of the clip path anyway in anticipation of user scrolling.
        // - We still want to reset needs_paint and request_paint flags.
        let child_mode = node.item.widget.paint_layer_mode();
        match child_mode {
            PaintLayerMode::Inline => paint_widget(
                global_state,
                default_properties,
                property_arena,
                current_scene,
                layers,
                scene_cache,
                current_layer,
                node.reborrow_mut(),
            ),
            PaintLayerMode::IsolatedScene | PaintLayerMode::External => {
                flush_scene_layer(
                    current_scene,
                    layers,
                    current_layer.boundary,
                    current_layer.bounds,
                    current_layer.clip,
                    current_layer.transform,
                    current_layer.root_id,
                );

                let child_transform = node.item.state.window_transform;
                let child_layer = LayerPaintState {
                    root_id: node.item.state.id,
                    boundary: VisualLayerBoundary::WidgetBoundary,
                    bounds: child_transform
                        .inverse()
                        .transform_rect_bbox(node.item.state.bounding_box),
                    clip: node.item.state.clip_path,
                    transform: child_transform,
                    window_to_layer_transform: child_transform.inverse(),
                };

                if child_mode == PaintLayerMode::IsolatedScene {
                    layers.extend(paint_subtree_as_layers(
                        global_state,
                        default_properties,
                        property_arena,
                        scene_cache,
                        child_layer,
                        node.reborrow_mut(),
                    ));
                } else {
                    let _ = paint_subtree_as_layers(
                        global_state,
                        default_properties,
                        property_arena,
                        scene_cache,
                        child_layer,
                        node.reborrow_mut(),
                    );
                    layers.push(VisualLayer::external(
                        ExternalLayerKind::Surface,
                        child_layer.boundary,
                        child_layer.bounds,
                        child_layer.clip,
                        child_layer.transform,
                        child_layer.root_id,
                    ));
                }
            }
        }
        parent_state.merge_up(&mut node.item.state);
    });

    if !is_stashed {
        if global_state.debug_paint {
            // Draw the global axis aligned bounding rect of the widget
            const BORDER_WIDTH: f64 = 1.0;
            let color = get_debug_color(id.to_raw());
            let rect = state.bounding_box.inset(BORDER_WIDTH / -2.0);
            let border_style = Stroke::new(BORDER_WIDTH);
            let mut painter = Painter::new(&mut *current_scene);
            painter.stroke(rect, &border_style, color).draw();

            // Draw the widget's explicit baselines
            let mut draw_baseline = |baseline| {
                let line = Line::new((0., baseline), (state.end_point.x, baseline));
                let baseline_style = Stroke::new(1.0).with_dashes(0., [4.0, 4.0]);
                painter
                    .stroke(line, &baseline_style, color)
                    .transform(border_box_to_layer_transform)
                    .draw();
            };
            if !state.first_baseline.is_nan() {
                draw_baseline(state.first_baseline);
            }
            if !state.last_baseline.is_nan() && state.last_baseline != state.first_baseline {
                draw_baseline(state.last_baseline);
            }
        }

        if has_clip {
            current_scene.pop_clip();
        }

        let Some((_, _, post_scene)) = &mut scene_cache.get(&id) else {
            debug_panic!(
                "Error in paint pass: scene should have been cached earlier in this function."
            );
            return;
        };

        current_scene.append_transformed(post_scene, content_box_to_layer_transform);
    }
}

// --- MARK: ROOT
/// See the [passes documentation](crate::doc::pass_system#render-passes).
pub(crate) fn run_paint_pass(root: &mut RenderRoot) -> VisualLayerPlan {
    let _span = info_span!("paint").entered();

    // TODO - This is a bit of a hack until we refactor widget tree mutation.
    // This should be removed once remove_child is exclusive to MutateCtx.
    let mut scene_cache = std::mem::take(&mut root.global_state.scene_cache);

    let root_id = root.root_id();
    let layer_root_ids = root.layer_root_ids();

    let mut layers = Vec::new();

    for (idx, &layer_widget_id) in layer_root_ids.iter().enumerate() {
        let (layer_to_window_transform, layer_bounds, layer_clip) = {
            let layer_state = root.widget_arena.get_state(layer_widget_id);
            let transform = layer_state.window_transform;
            let bounds = transform
                .inverse()
                .transform_rect_bbox(layer_state.bounding_box);
            (transform, bounds, layer_state.clip_path)
        };
        if idx == 0 {
            layers.extend(paint_layer(
                root,
                &mut scene_cache,
                root_id,
                layer_widget_id,
                VisualLayerBoundary::LayerRoot,
                layer_bounds,
                layer_clip,
                layer_to_window_transform,
            ));
            continue;
        }

        let layer_realization = root
            .widget_arena
            .get_node_mut(layer_widget_id)
            .item
            .widget
            .as_layer()
            .map(|layer| layer.realization())
            .unwrap_or(LayerRealization::Scene);

        if layer_realization == LayerRealization::External {
            // Still run the paint traversal so we clear paint flags and keep cache state
            // consistent, but the resulting retained scene is intentionally discarded.
            let _ = paint_layer(
                root,
                &mut scene_cache,
                root_id,
                layer_widget_id,
                VisualLayerBoundary::LayerRoot,
                layer_bounds,
                layer_clip,
                layer_to_window_transform,
            );
            layers.push(VisualLayer::external(
                ExternalLayerKind::Surface,
                VisualLayerBoundary::LayerRoot,
                layer_bounds,
                layer_clip,
                layer_to_window_transform,
                layer_widget_id,
            ));
        } else {
            layers.extend(paint_layer(
                root,
                &mut scene_cache,
                root_id,
                layer_widget_id,
                VisualLayerBoundary::LayerRoot,
                layer_bounds,
                layer_clip,
                layer_to_window_transform,
            ));
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

        // Draw the hover rect in the owning layer's most-recent scene chunk.
        if let Some(layer) = layers
            .iter_mut()
            .rev()
            .find(|layer| layer.root_id == layer_root)
            && let VisualLayerKind::Scene(scene) = &mut layer.kind
        {
            Painter::new(scene)
                .fill(rect, HOVER_FILL_COLOR)
                .transform(border_box_to_layer_transform)
                .draw();
        }
    }

    VisualLayerPlan::new(layers)
}

/// Paint a single layer root into ordered render layers.
///
/// This is a helper that handles the split borrows needed to access
/// `global_state`, `default_properties`, and `widget_arena` simultaneously.
fn paint_layer(
    root: &mut RenderRoot,
    scene_cache: &mut HashMap<WidgetId, (Scene, Scene, Scene)>,
    root_id: WidgetId,
    layer_widget_id: WidgetId,
    layer_boundary: VisualLayerBoundary,
    layer_bounds: Rect,
    layer_clip: Option<Rect>,
    layer_transform: Affine,
) -> Vec<VisualLayer> {
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
        return Vec::new();
    };

    let layer_state = LayerPaintState {
        root_id: layer_widget_id,
        boundary: layer_boundary,
        bounds: layer_bounds,
        clip: layer_clip,
        transform: layer_transform,
        window_to_layer_transform: layer_node.item.state.window_transform.inverse(),
    };
    let paint_layer_mode = layer_node.item.widget.paint_layer_mode();

    if paint_layer_mode == PaintLayerMode::External {
        let _ = paint_subtree_as_layers(
            &mut root.global_state,
            &root.property_arena.default_properties,
            &root.property_arena,
            scene_cache,
            layer_state,
            layer_node,
        );
        vec![VisualLayer::external(
            ExternalLayerKind::Surface,
            layer_state.boundary,
            layer_state.bounds,
            layer_state.clip,
            layer_state.transform,
            layer_state.root_id,
        )]
    } else {
        paint_subtree_as_layers(
            &mut root.global_state,
            &root.property_arena.default_properties,
            &root.property_arena,
            scene_cache,
            layer_state,
            layer_node,
        )
    }
}

#[cfg(test)]
mod tests;
