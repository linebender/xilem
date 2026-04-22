// Copyright 2024 the Xilem Authors
// SPDX-License-Identifier: Apache-2.0

use std::collections::HashMap;

use kurbo::{Affine, Line, Stroke};
use peniko::{Color, Fill};
use tracing::{info_span, trace};
use tree_arena::ArenaMut;

use crate::app::{
    AppCtx, RenderRoot, RenderRootState, VisualLayer, VisualLayerKind, VisualLayerPlan,
};
use crate::core::{
    DefaultProperties, PaintCtx, PaintLayerMode, PropertiesRef, PropertyArena, WidgetArenaNode,
    WidgetId,
};
use crate::imaging::record::{Clip, Geometry, Scene};
use crate::imaging::{PaintSink, Painter};
use crate::passes::{enter_span_if, recurse_on_children};
use crate::util::get_debug_color;

struct LayerCollector {
    current_scene: Scene,
    layers: Vec<VisualLayer>,
    current_owner_id: WidgetId,
    transform: Affine,
}

impl LayerCollector {
    fn new(root_id: WidgetId, transform: Affine) -> Self {
        Self {
            current_scene: Scene::new(),
            layers: Vec::new(),
            current_owner_id: root_id,
            transform,
        }
    }

    fn scene_mut(&mut self) -> &mut Scene {
        &mut self.current_scene
    }

    fn finish_current_layer(&mut self, allow_empty: bool) {
        let empty_scene = Scene::new();
        if !allow_empty && self.current_scene == empty_scene {
            return;
        }

        let scene = std::mem::replace(&mut self.current_scene, empty_scene);
        self.layers.push(VisualLayer {
            kind: VisualLayerKind::Scene(scene),
            transform: self.transform,
            widget_id: self.current_owner_id,
        });
    }

    fn push_external_layer(&mut self, widget_id: WidgetId, bounds: kurbo::Rect) {
        self.layers.push(VisualLayer {
            kind: VisualLayerKind::External { bounds },
            transform: self.transform,
            widget_id,
        });
    }

    fn into_layers(mut self) -> Vec<VisualLayer> {
        self.finish_current_layer(self.layers.is_empty());
        self.layers
    }
}

// --- MARK: PAINT WIDGET
fn paint_widget(
    app_ctx: &mut AppCtx,
    global_state: &mut RenderRootState,
    default_properties: &DefaultProperties,
    property_arena: &PropertyArena,
    layer_collector: &mut LayerCollector,
    scene_cache: &mut HashMap<WidgetId, (Scene, Scene, Scene)>,
    window_to_layer_transform: &Affine,
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

    state.paint_layer_mode = PaintLayerMode::Inline;
    if (state.request_pre_paint || state.request_paint || state.request_post_paint) && !is_stashed {
        if trace {
            trace!("Painting widget '{}' {}", widget.short_type_name(), id);
        }

        let stack = property_arena.get(state.property_stack_id, widget.type_id());
        let mut ctx = PaintCtx {
            app_ctx,
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

    let paint_layer_mode = if is_stashed {
        PaintLayerMode::Inline
    } else {
        state.paint_layer_mode
    };

    if matches!(
        paint_layer_mode,
        PaintLayerMode::IsolatedScene | PaintLayerMode::External
    ) {
        layer_collector.finish_current_layer(false);
    }

    let previous_owner_id = layer_collector.current_owner_id;
    layer_collector.current_owner_id = match paint_layer_mode {
        PaintLayerMode::Inline => previous_owner_id,
        PaintLayerMode::IsolatedScene | PaintLayerMode::External => id,
    };

    let border_box_to_layer_transform = *window_to_layer_transform * state.window_transform;
    let content_box_to_layer_transform =
        border_box_to_layer_transform.pre_translate(state.border_box_translation());
    let has_clip = state.clip_path.is_some();
    let paint_as_external = paint_layer_mode == PaintLayerMode::External;

    if !is_stashed && !paint_as_external {
        let Some((pre_scene, scene, _)) = &mut scene_cache.get(&id) else {
            debug_panic!(
                "Error in paint pass: scene should have been cached earlier in this function."
            );
            return;
        };

        layer_collector
            .scene_mut()
            .append_transformed(pre_scene, content_box_to_layer_transform);

        if let Some(clip) = state.clip_path {
            // The clip path is stored in border-box space, so need to use that transform.
            layer_collector.scene_mut().push_clip(Clip::Fill {
                transform: border_box_to_layer_transform,
                shape: Geometry::Rect(clip),
                fill_rule: Fill::NonZero,
            });
        }

        layer_collector
            .scene_mut()
            .append_transformed(scene, content_box_to_layer_transform);
    }

    let parent_state = &mut *state;
    recurse_on_children(id, widget, children, |mut node| {
        // TODO: We could skip painting children outside the parent clip path.
        // There's a few things to consider if we do:
        // - Some widgets can paint outside of their layout box.
        // - Once we implement compositor layers, we may want to paint outside of the clip path anyway in anticipation of user scrolling.
        // - We still want to reset needs_paint and request_paint flags.
        paint_widget(
            app_ctx,
            global_state,
            default_properties,
            property_arena,
            layer_collector,
            scene_cache,
            window_to_layer_transform,
            node.reborrow_mut(),
        );
        parent_state.merge_up(&mut node.item.state);
    });

    if !is_stashed && !paint_as_external {
        if global_state.debug_paint {
            // Draw the global axis aligned bounding rect of the widget
            const BORDER_WIDTH: f64 = 1.0;
            let color = get_debug_color(id.to_raw());
            let rect = state.bounding_box.inset(BORDER_WIDTH / -2.0);
            let border_style = Stroke::new(BORDER_WIDTH);
            let mut painter = Painter::new(layer_collector.scene_mut());
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
            layer_collector.scene_mut().pop_clip();
        }

        let Some((_, _, post_scene)) = &mut scene_cache.get(&id) else {
            debug_panic!(
                "Error in paint pass: scene should have been cached earlier in this function."
            );
            return;
        };

        layer_collector
            .scene_mut()
            .append_transformed(post_scene, content_box_to_layer_transform);

        if global_state.inspector_state.hovered_widget == Some(id) {
            const HOVER_FILL_COLOR: Color = Color::from_rgba8(60, 60, 250, 100);
            let rect = state.border_box_size().to_rect();
            Painter::new(layer_collector.scene_mut())
                .fill(rect, HOVER_FILL_COLOR)
                .transform(border_box_to_layer_transform)
                .draw();
        }
    }

    if paint_as_external {
        layer_collector.push_external_layer(id, state.border_box_size().to_rect());
    }

    if matches!(
        paint_layer_mode,
        PaintLayerMode::IsolatedScene | PaintLayerMode::External
    ) {
        layer_collector.finish_current_layer(false);
    }

    layer_collector.current_owner_id = previous_owner_id;
}

// --- MARK: ROOT
/// See the [passes documentation](crate::doc::pass_system#render-passes).
pub(crate) fn run_paint_pass(app_ctx: &mut AppCtx, root: &mut RenderRoot) -> VisualLayerPlan {
    let _span = info_span!("paint").entered();

    // TODO - This is a bit of a hack until we refactor widget tree mutation.
    // This should be removed once remove_child is exclusive to MutateCtx.
    let mut scene_cache = std::mem::take(&mut root.global_state.scene_cache);

    let root_id = root.root_id();
    let layer_root_ids = root.layer_root_ids();

    let mut layers = Vec::new();

    for (idx, &layer_widget_id) in layer_root_ids.iter().enumerate() {
        let transform = if idx == 0 {
            Affine::IDENTITY
        } else {
            root.widget_arena
                .get_state(layer_widget_id)
                .window_transform
        };

        let mut collector = LayerCollector::new(layer_widget_id, transform);
        paint_layer(
            app_ctx,
            root,
            &mut collector,
            &mut scene_cache,
            root_id,
            layer_widget_id,
        );
        layers.extend(collector.into_layers());
    }

    root.global_state.scene_cache = scene_cache;

    VisualLayerPlan { layers }
}

/// Paint a single layer's widget subtree into `target_scene`.
///
/// This is a helper that handles the split borrows needed to access
/// `global_state`, `default_properties`, and `widget_arena` simultaneously.
fn paint_layer(
    app_ctx: &mut AppCtx,
    root: &mut RenderRoot,
    layer_collector: &mut LayerCollector,
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
        app_ctx,
        &mut root.global_state,
        &root.property_arena.default_properties,
        &root.property_arena,
        layer_collector,
        scene_cache,
        &window_to_layer_transform,
        layer_node,
    );
}
