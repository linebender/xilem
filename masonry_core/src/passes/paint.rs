// Copyright 2024 the Xilem Authors
// SPDX-License-Identifier: Apache-2.0

use std::collections::HashMap;

use tracing::{info_span, trace};
use vello::Scene;
use vello::kurbo::{Affine, Rect};
use vello::peniko::{Color, Fill, Mix};

use crate::app::{RenderRoot, RenderRootState};
use crate::core::{
    DefaultProperties, PaintCtx, PropertiesRef, WidgetArenaMut, WidgetId, WidgetItemMut,
};
use crate::passes::enter_span_if;
use crate::util::{get_debug_color, stroke};

// --- MARK: PAINT WIDGET
fn paint_widget(
    global_state: &mut RenderRootState,
    default_properties: &DefaultProperties,
    complete_scene: &mut Scene,
    scenes: &mut HashMap<WidgetId, Scene>,
    item: WidgetItemMut<'_>,
    children: WidgetArenaMut<'_>,
    debug_paint: bool,
) {
    let trace = global_state.trace.paint;
    let _span = enter_span_if(trace, &**item.widget, item.state.id);

    let id = item.state.id;

    // TODO - Handle damage regions
    // https://github.com/linebender/xilem/issues/789
    let mut ctx = PaintCtx {
        global_state,
        widget_state: item.state,
        debug_paint,
    };
    if ctx.widget_state.request_paint {
        if trace {
            trace!("Painting widget '{}' {}", item.widget.short_type_name(), id);
        }

        // TODO - Reserve scene
        // https://github.com/linebender/xilem/issues/524
        let scene = scenes.entry(id).or_default();
        scene.reset();
        let props = PropertiesRef {
            map: item.properties,
            default_map: default_properties.for_widget(item.widget.type_id()),
        };
        item.widget.paint(&mut ctx, &props, scene);
    }

    item.state.request_paint = false;
    item.state.needs_paint = false;

    let clip = item.state.clip_path;
    let has_clip = clip.is_some();
    let transform = item.state.window_transform;
    let scene = scenes.get(&id).unwrap();

    if let Some(clip) = clip {
        complete_scene.push_layer(Mix::Clip, 1., transform, &clip);
    }

    complete_scene.append(scene, Some(transform));

    let id = item.state.id;
    let bounding_rect = item.state.bounding_rect;
    let parent_state = item.state;
    crate::passes::recurse_on_children2(id, &**item.widget, children, |mut item, children| {
        // TODO - We skip painting stashed items.
        // This may lead to zombie flags in rare cases, we need to fix this.
        if item.state.is_stashed {
            return;
        }
        // TODO: We could skip painting children outside the parent clip path.
        // There's a few things to consider if we do:
        // - Some widgets can paint outside of their layout box.
        // - Once we implement compositor layers, we may want to paint outside of the clip path anyway in anticipation of user scrolling.
        paint_widget(
            global_state,
            default_properties,
            complete_scene,
            scenes,
            item.reborrow_mut(),
            children,
            debug_paint,
        );
        parent_state.merge_up(item.state);
    });

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

// --- MARK: ROOT
/// See the [passes documentation](../doc/05_pass_system.md#render-passes).
pub(crate) fn run_paint_pass(root: &mut RenderRoot) -> Scene {
    let _span = info_span!("paint").entered();

    // TODO - Reserve scene
    // https://github.com/linebender/xilem/issues/524
    let mut complete_scene = Scene::new();

    let (root_item, root_children) = root.widget_arena.get_mut(root.root.id());

    // TODO - This is a bit of a hack until we refactor widget tree mutation.
    // This should be removed once remove_child is exclusive to MutateCtx.
    let mut scenes = std::mem::take(&mut root.global_state.scenes);

    paint_widget(
        &mut root.global_state,
        &root.default_properties,
        &mut complete_scene,
        &mut scenes,
        root_item,
        root_children,
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
