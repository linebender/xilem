// Copyright 2024 the Xilem Authors
// SPDX-License-Identifier: Apache-2.0

use std::collections::HashMap;

use tracing::{info_span, trace};
use tree_arena::ArenaMut;
use vello::Scene;
use vello::kurbo::{Affine, Rect};
use vello::peniko::{Color, Fill, Mix};

use crate::app::{RenderRoot, RenderRootState};
use crate::core::{
    DefaultProperties, PaintCtx, PropertiesRef, Widget, WidgetArenaMut, WidgetId, WidgetState,
};
use crate::passes::{enter_span_if, recurse_on_children};
use crate::util::{AnyMap, get_debug_color, stroke};

// --- MARK: PAINT WIDGET
fn paint_widget(
    global_state: &mut RenderRootState,
    default_properties: &DefaultProperties,
    complete_scene: &mut Scene,
    scenes: &mut HashMap<WidgetId, Scene>,
    widget: ArenaMut<'_, Box<dyn Widget>>,
    mut state: ArenaMut<'_, WidgetState>,
    properties: ArenaMut<'_, AnyMap>,
    debug_paint: bool,
) {
    let trace = global_state.trace.paint;
    let _span = enter_span_if(trace, state.reborrow());

    let id = state.item.id;
    let is_stashed = state.item.is_stashed;

    // Note: At this point we could short-circuit if is_stashed is true,
    // but we deliberately avoid doing that to avoid creating zombie flags.
    // (See WidgetState doc.)

    let mut children = WidgetArenaMut {
        widget_children: widget.children,
        widget_state_children: state.children,
        properties_children: properties.children,
    };
    let widget = &mut **widget.item;
    let state = state.item;
    let properties = properties.item;

    // TODO - Handle damage regions
    // https://github.com/linebender/xilem/issues/789
    let mut ctx = PaintCtx {
        global_state,
        widget_state: state,
        children: children.reborrow_mut(),
        debug_paint,
    };
    if ctx.widget_state.request_paint && !is_stashed {
        if trace {
            trace!("Painting widget '{}' {}", widget.short_type_name(), id);
        }

        // TODO - Reserve scene
        // https://github.com/linebender/xilem/issues/524
        let scene = scenes.entry(id).or_default();
        scene.reset();
        let props = PropertiesRef {
            map: properties,
            default_map: default_properties.for_widget(widget.type_id()),
        };
        widget.paint(&mut ctx, &props, scene);
    }

    state.request_paint = false;
    state.needs_paint = false;

    let has_clip = state.clip_path.is_some();
    if !is_stashed {
        let transform = state.window_transform;
        let scene = scenes.get(&id).unwrap();

        if let Some(clip) = state.clip_path {
            complete_scene.push_layer(Mix::Clip, 1., transform, &clip);
        }

        complete_scene.append(scene, Some(transform));
    }

    let bounding_rect = state.bounding_rect;
    let parent_state = state;

    recurse_on_children(id, widget, children, |widget, mut state, properties| {
        // TODO: We could skip painting children outside the parent clip path.
        // There's a few things to consider if we do:
        // - Some widgets can paint outside of their layout box.
        // - Once we implement compositor layers, we may want to paint outside of the clip path anyway in anticipation of user scrolling.
        // - We still want to reset needs_paint and request_paint flags.
        paint_widget(
            global_state,
            default_properties,
            complete_scene,
            scenes,
            widget,
            state.reborrow_mut(),
            properties,
            debug_paint,
        );
        parent_state.merge_up(state.item);
    });

    if !is_stashed {
        // draw the global axis aligned bounding rect of the widget
        if debug_paint {
            const BORDER_WIDTH: f64 = 1.0;
            let color = get_debug_color(id.to_raw());
            let rect = bounding_rect.inset(BORDER_WIDTH / -2.0);
            stroke(complete_scene, &rect, color, BORDER_WIDTH);
        }

        if has_clip {
            complete_scene.pop_layer();
        }
    }
}

// --- MARK: ROOT
/// See the [passes documentation](../doc/05_pass_system.md#render-passes).
pub(crate) fn run_paint_pass(root: &mut RenderRoot) -> Scene {
    let _span = info_span!("paint").entered();

    // TODO - Reserve scene
    // https://github.com/linebender/xilem/issues/524
    let mut complete_scene = Scene::new();

    let (root_widget, root_state, root_properties) = {
        let widget_id = root.root.id();
        let widget = root
            .widget_arena
            .widgets
            .find_mut(widget_id)
            .expect("root_paint: root not in widget tree");
        let state = root
            .widget_arena
            .states
            .find_mut(widget_id)
            .expect("root_paint: root state not in widget tree");
        let properties = root
            .widget_arena
            .properties
            .find_mut(widget_id)
            .expect("root_paint: root properties not in widget tree");
        (widget, state, properties)
    };

    // TODO - This is a bit of a hack until we refactor widget tree mutation.
    // This should be removed once remove_child is exclusive to MutateCtx.
    let mut scenes = std::mem::take(&mut root.global_state.scenes);

    paint_widget(
        &mut root.global_state,
        &root.default_properties,
        &mut complete_scene,
        &mut scenes,
        root_widget,
        root_state,
        root_properties,
        root.debug_paint,
    );
    root.global_state.scenes = scenes;

    // Display a rectangle over the hovered widget
    if let Some(hovered_widget) = root.global_state.inspector_state.hovered_widget {
        const HOVER_FILL_COLOR: Color = Color::from_rgba8(60, 60, 250, 100);
        let state = root.widget_arena.get_state(hovered_widget).item;
        let rect = Rect::from_origin_size(state.window_origin(), state.size);

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
