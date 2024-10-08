// Copyright 2024 the Xilem Authors
// SPDX-License-Identifier: Apache-2.0

use std::collections::HashMap;

use tracing::{info_span, trace};
use vello::kurbo::{Affine, Stroke};
use vello::peniko::Mix;
use vello::Scene;

use crate::passes::recurse_on_children;
use crate::render_root::{RenderRoot, RenderRootState};
use crate::theme::get_debug_color;
use crate::tree_arena::ArenaMut;
use crate::{PaintCtx, Widget, WidgetId, WidgetState};

// --- MARK: PAINT WIDGET ---
fn paint_widget(
    global_state: &mut RenderRootState,
    complete_scene: &mut Scene,
    scenes: &mut HashMap<WidgetId, Scene>,
    mut widget: ArenaMut<'_, Box<dyn Widget>>,
    mut state: ArenaMut<'_, WidgetState>,
    debug_paint: bool,
) {
    let _span = widget.item.make_trace_span().entered();
    let id = state.item.id;

    // TODO - Handle invalidation regions
    let mut ctx = PaintCtx {
        global_state,
        widget_state: state.item,
        widget_state_children: state.children.reborrow_mut(),
        widget_children: widget.children.reborrow_mut(),
        debug_paint,
    };
    if ctx.widget_state.request_paint {
        trace!("Painting widget '{}' {}", widget.item.short_type_name(), id,);

        // TODO - Reserve scene
        // https://github.com/linebender/xilem/issues/524
        let scene = scenes.entry(id).or_default();
        scene.reset();
        widget.item.paint(&mut ctx, scene);
    }

    state.item.request_paint = false;
    state.item.needs_paint = false;

    let clip = state.item.clip;
    let has_clip = clip.is_some();
    let transform = Affine::translate(state.item.window_origin.to_vec2());
    let scene = scenes.get(&id).unwrap();

    if let Some(clip) = clip {
        complete_scene.push_layer(Mix::Clip, 1., transform, &clip);
    }

    complete_scene.append(scene, Some(transform));

    let id = state.item.id;
    let size = state.item.size;
    let parent_state = state.item;
    recurse_on_children(
        id,
        widget.reborrow_mut(),
        state.children,
        |widget, mut state| {
            // TODO - We skip painting stashed items.
            // This may have knock-on effects we'd need to document.
            if state.item.is_stashed {
                return;
            }
            // TODO: We could skip painting children outside the parent clip path.
            // There's a few things to consider if we do:
            // - Some widgets can paint outside of their layout box.
            // - Once we implement compositor layers, we may want to paint outside of the clip path anyway in anticipation of user scrolling.
            paint_widget(
                global_state,
                complete_scene,
                scenes,
                widget,
                state.reborrow_mut(),
                debug_paint,
            );
            parent_state.merge_up(state.item);
        },
    );

    if debug_paint {
        const BORDER_WIDTH: f64 = 1.0;
        let rect = size.to_rect().inset(BORDER_WIDTH / -2.0);
        let color = get_debug_color(id.to_raw());
        complete_scene.stroke(&Stroke::new(BORDER_WIDTH), transform, color, None, &rect);
    }

    if has_clip {
        complete_scene.pop_layer();
    }
}

// --- MARK: ROOT ---
pub(crate) fn root_paint(root: &mut RenderRoot) -> Scene {
    let _span = info_span!("paint").entered();

    let debug_paint = std::env::var("MASONRY_DEBUG_PAINT").is_ok_and(|it| !it.is_empty());

    // TODO - Reserve scene
    // https://github.com/linebender/xilem/issues/524
    let mut complete_scene = Scene::new();

    let (root_widget, root_state) = {
        let widget_id = root.root.id();
        let widget = root
            .widget_arena
            .widgets
            .find_mut(widget_id.to_raw())
            .expect("root_paint: root not in widget tree");
        let state = root
            .widget_arena
            .widget_states
            .find_mut(widget_id.to_raw())
            .expect("root_paint: root state not in widget tree");
        (widget, state)
    };

    // TODO - This is a bit of a hack until we refactor widget tree mutation.
    // This should be removed once remove_child is exclusive to MutateCtx.
    let mut scenes = std::mem::take(&mut root.state.scenes);

    paint_widget(
        &mut root.state,
        &mut complete_scene,
        &mut scenes,
        root_widget,
        root_state,
        debug_paint,
    );
    root.state.scenes = scenes;

    complete_scene
}
