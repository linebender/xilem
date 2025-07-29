// Copyright 2025 the Xilem Authors
// SPDX-License-Identifier: Apache-2.0

use tracing::{info_span, trace};
use tree_arena::ArenaMut;

use crate::app::{RenderRoot, RenderRootState};
use crate::core::{
    DefaultProperties, PropertiesRef, QueryCtx, Widget, WidgetArenaMut, WidgetId, WidgetState,
};
use crate::passes::recurse_on_children;
use crate::util::AnyMap;

// --- MARK: QUERY
fn run_query_on(
    global_state: &mut RenderRootState,
    default_properties: &DefaultProperties,
    pred: &mut impl FnMut(&dyn Widget, QueryCtx<'_>) -> bool,
    widget: ArenaMut<'_, Box<dyn Widget>>,
    state: ArenaMut<'_, WidgetState>,
    properties: ArenaMut<'_, AnyMap>,
    debug_paint: bool,
) -> Option<WidgetId> {
    let id = state.item.id;

    let children = WidgetArenaMut {
        widget_children: widget.children,
        widget_state_children: state.children,
        properties_children: properties.children,
    };
    let widget = &mut **widget.item;
    let state = state.item;
    let properties = properties.item;

    let ctx = QueryCtx {
        global_state,
        widget_state: state,
        properties: PropertiesRef {
            map: properties,
            default_map: default_properties.for_widget(widget.type_id()),
        },
        children: children.reborrow(),
    };

    let mut found = None;
    if pred(widget, ctx) {
        found = Some(id);
    }

    recurse_on_children(id, widget, children, |widget, state, properties| {
        if found.is_some() {
            return;
        }

        found = run_query_on(
            global_state,
            default_properties,
            pred,
            widget,
            state,
            properties,
            debug_paint,
        );
    });

    found
}

// --- MARK: ROOT
/// This is not a pass in the same sense as rewrite and render passes,
/// but it does recursively iterate over the widget tree.
// TODO - Take &RenderRoot instead
pub(crate) fn run_query(
    root: &mut RenderRoot,
    mut pred: impl FnMut(&dyn Widget, QueryCtx<'_>) -> bool,
) -> Option<WidgetId> {
    let _span = info_span!("query").entered();

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

    let found = run_query_on(
        &mut root.global_state,
        &root.default_properties,
        &mut pred,
        root_widget,
        root_state,
        root_properties,
        root.debug_paint,
    );

    if let Some(id) = found {
        trace!("found widget {id}");
    }

    found
}
