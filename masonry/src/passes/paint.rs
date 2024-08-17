// Copyright 2024 the Xilem Authors
// SPDX-License-Identifier: Apache-2.0

use std::collections::HashMap;

use tracing::{info_span, trace};
use vello::kurbo::{Affine, Rect, Stroke};
use vello::peniko::BlendMode;
use vello::Scene;

use crate::passes::recurse_on_children;
use crate::render_root::{RenderRoot, RenderRootState};
use crate::theme::get_debug_color;
use crate::tree_arena::ArenaMut;
use crate::{PaintCtx, Widget, WidgetId, WidgetState};

fn paint_widget(
    global_state: &mut RenderRootState,
    complete_scene: &mut Scene,
    scenes: &mut HashMap<WidgetId, Scene>,
    mut widget: ArenaMut<'_, Box<dyn Widget>>,
    mut state: ArenaMut<'_, WidgetState>,
    depth: u32,
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
        depth,
        debug_paint,
    };
    if ctx.widget_state.request_paint {
        trace!(
            "Painting widget '{}' #{}",
            widget.item.short_type_name(),
            id.to_raw(),
        );

        // TODO - Reserve scene
        // https://github.com/linebender/xilem/issues/524
        let mut scene = Scene::new();
        widget.item.paint(&mut ctx, &mut scene);
        *scenes.entry(id).or_default() = scene;
    }

    state.item.request_paint = false;
    state.item.needs_paint = false;

    // TODO
    let clip: Option<Rect> = None;
    let has_clip = clip.is_some();
    let transform = Affine::translate(dbg!(state.item.window_origin).to_vec2());
    let scene = scenes.get(&id).unwrap();

    if let Some(clip) = clip {
        complete_scene.push_layer(BlendMode::default(), 1., transform, &clip);
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
            paint_widget(
                global_state,
                complete_scene,
                scenes,
                widget,
                state.reborrow_mut(),
                depth + 1,
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

// ----------------

pub fn root_paint(root: &mut RenderRoot) -> Scene {
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

    paint_widget(
        &mut root.state,
        &mut complete_scene,
        &mut root.widget_arena.scenes,
        root_widget,
        root_state,
        0,
        debug_paint,
    );

    complete_scene
}
