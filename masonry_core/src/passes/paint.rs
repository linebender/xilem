// Copyright 2024 the Xilem Authors
// SPDX-License-Identifier: Apache-2.0

use std::collections::HashMap;

use tracing::{info_span, trace};
use tree_arena::ArenaMut;
use vello::Scene;
use vello::kurbo::{Affine, Rect};
use vello::peniko::{Color, Fill};

use crate::app::{RenderRoot, RenderRootState};
use crate::core::{DefaultProperties, PaintCtx, PropertiesRef, WidgetArenaNode, WidgetId};
use crate::passes::{enter_span_if, recurse_on_children};
use crate::properties::{
    ActiveBackground, Background, BorderWidth, BoxShadow, CornerRadius, DisabledBackground,
};
use crate::util::{get_debug_color, stroke};

// --- MARK: PAINT WIDGET
fn paint_widget(
    global_state: &mut RenderRootState,
    default_properties: &DefaultProperties,
    complete_scene: &mut Scene,
    scene_cache: &mut HashMap<WidgetId, (Scene, Scene)>,
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

    if !is_stashed {
        if trace {
            trace!("Painting widget '{}' {}", widget.short_type_name(), id);
        }

        let props = PropertiesRef {
            map: properties,
            default_map: default_properties.for_widget(widget.type_id()),
        };

        let size = state.size();
        let border_width = props.get::<BorderWidth>();
        let border_radius = props.get::<CornerRadius>();
        let transform = state.window_transform;

        // Paint box shadow
        let shadow = props.get::<BoxShadow>();
        if shadow.is_visible() {
            let shadow_rect = shadow.shadow_rect(size, border_radius);
            shadow.paint(complete_scene, transform, shadow_rect);
        }

        // Paint background
        let bg = if state.is_disabled {
            &props.get::<DisabledBackground>().0
        } else if state.is_active {
            &props.get::<ActiveBackground>().0
        } else {
            props.get::<Background>()
        };
        // TODO: Fix remaining issues, see https://github.com/linebender/xilem/issues/1592
        //    1. Figure out how to skip painting fully transparent backgrounds.
        //    2. Don't subtract the border from the background rect. Will need solution for border
        //       painting, as background should go exactly to the outer border and not beyond.
        let bg_rect = border_width.bg_rect(size, border_radius);
        let bg_brush = bg.get_peniko_brush_for_rect(bg_rect.rect());
        complete_scene.fill(Fill::NonZero, transform, &bg_brush, None, &bg_rect);

        if state.request_paint || state.request_post_paint {
            let mut ctx = PaintCtx {
                global_state,
                widget_state: state,
                children: children.reborrow_mut(),
            };

            // TODO - Reserve scene
            // https://github.com/linebender/xilem/issues/524
            let (scene, postfix_scene) = scene_cache.entry(id).or_default();

            if state.request_paint {
                scene.reset();
                widget.paint(&mut ctx, &props, scene);
            }
            if state.request_post_paint {
                postfix_scene.reset();
                widget.post_paint(&mut ctx, &props, postfix_scene);
            }
        }
    }

    state.request_paint = false;
    state.request_post_paint = false;
    state.needs_paint = false;

    let has_clip = state.clip_path.is_some();
    if !is_stashed {
        let transform = state.window_transform;
        let Some((scene, _)) = &mut scene_cache.get(&id) else {
            debug_panic!(
                "Error in paint pass: scene should have been cached earlier in this function."
            );
            return;
        };

        if let Some(clip) = state.clip_path {
            complete_scene.push_clip_layer(Fill::NonZero, transform, &clip);
        }

        complete_scene.append(scene, Some(transform));
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
            node.reborrow_mut(),
        );
        parent_state.merge_up(&mut node.item.state);
    });

    if !is_stashed {
        let transform = state.window_transform;
        let bounding_rect = state.bounding_rect;

        // draw the global axis aligned bounding rect of the widget
        if global_state.debug_paint {
            const BORDER_WIDTH: f64 = 1.0;
            let color = get_debug_color(id.to_raw());
            let rect = bounding_rect.inset(BORDER_WIDTH / -2.0);
            stroke(complete_scene, &rect, color, BORDER_WIDTH);
        }

        if has_clip {
            complete_scene.pop_layer();
        }

        let Some((_, postfix_scene)) = &mut scene_cache.get(&id) else {
            debug_panic!(
                "Error in paint pass: scene should have been cached earlier in this function."
            );
            return;
        };

        complete_scene.append(postfix_scene, Some(transform));
    }
}

// --- MARK: ROOT
/// See the [passes documentation](crate::doc::pass_system#render-passes).
pub(crate) fn run_paint_pass(root: &mut RenderRoot) -> Scene {
    let _span = info_span!("paint").entered();

    // TODO - Reserve scene
    // https://github.com/linebender/xilem/issues/524
    let mut complete_scene = Scene::new();

    let root_node = root.widget_arena.get_node_mut(root.root_id());

    // TODO - This is a bit of a hack until we refactor widget tree mutation.
    // This should be removed once remove_child is exclusive to MutateCtx.
    let mut scene_cache = std::mem::take(&mut root.global_state.scene_cache);

    paint_widget(
        &mut root.global_state,
        &root.default_properties,
        &mut complete_scene,
        &mut scene_cache,
        root_node,
    );
    root.global_state.scene_cache = scene_cache;

    // Display a rectangle over the hovered widget
    if let Some(hovered_widget) = root.global_state.inspector_state.hovered_widget {
        const HOVER_FILL_COLOR: Color = Color::from_rgba8(60, 60, 250, 100);
        let state = root.widget_arena.get_state(hovered_widget);
        let rect = Rect::from_origin_size(state.window_origin(), state.size());

        complete_scene.fill(
            Fill::NonZero,
            Affine::IDENTITY,
            HOVER_FILL_COLOR,
            None,
            &rect,
        );
    }

    complete_scene
}
