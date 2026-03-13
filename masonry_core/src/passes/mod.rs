// Copyright 2024 the Xilem Authors
// SPDX-License-Identifier: Apache-2.0

//! Internal passes run by Masonry every frame.
//!
//! See the [passes documentation](crate::doc::pass_system) for more information.
//!
//! This file includes utility functions used by multiple passes.

use tracing::span::EnteredSpan;
use tree_arena::{ArenaMut, ArenaMutList};

use crate::core::{Widget, WidgetArena, WidgetArenaNode, WidgetId, WidgetState};

pub(crate) mod accessibility;
pub(crate) mod anim;
pub(crate) mod compose;
pub(crate) mod event;
pub(crate) mod layout;
pub(crate) mod mutate;
pub(crate) mod paint;
pub(crate) mod update;

#[must_use = "Span will be immediately closed if dropped"]
pub(crate) fn enter_span_if(enabled: bool, state: &WidgetState) -> Option<EnteredSpan> {
    enabled.then(|| enter_span(state))
}

#[must_use = "Span will be immediately closed if dropped"]
pub(crate) fn enter_span(state: &WidgetState) -> EnteredSpan {
    state.trace_span.clone().entered()
}

pub(crate) fn recurse_on_children(
    id: WidgetId,
    widget: &dyn Widget,
    mut children: ArenaMutList<'_, WidgetArenaNode>,
    mut callback: impl FnMut(ArenaMut<'_, WidgetArenaNode>),
) {
    let parent_name = widget.short_type_name();
    let parent_id = id;

    for child_id in widget.children_ids() {
        let Some(node) = children.item_mut(child_id) else {
            panic!(
                "Error in '{parent_name}' {parent_id}: cannot find child {child_id} returned by children_ids()"
            );
        };

        callback(node);
    }
}

pub(crate) fn merge_state_up(arena: &mut WidgetArena, widget_id: WidgetId) {
    let parent_id = arena.parent_of(widget_id);

    let Some(parent_id) = parent_id else {
        // We've reached the root
        return;
    };

    let mut parent_node_mut = arena.nodes.find_mut(parent_id).unwrap();
    let child_node_mut = parent_node_mut.children.item_mut(widget_id).unwrap();

    parent_node_mut
        .item
        .state
        .merge_up(&mut child_node_mut.item.state);
}

/// Masonry has a significant number of passes which may traverse a significant number of
/// items.
///
/// In most cases, including these elements in traces adds noise and makes operations extremely slow.
/// Because of this, we default these traces to false.
///
/// Using the default tracing filtering mechanism for this would be non-ideal, as it would prevent child
/// spans of the item from running, which may make end-user debugging harder.
///
/// The detailed traces for these passes therefore default to false, but can be enabled using the
/// `MASONRY_TRACE_PASSES` environment variable.
///
/// Note that passes which are bounded by depth (rather than absolute size) are never filtered out here.
///
/// Ideally, we'd cache the spans, which would make a lot (but not all) of this unnecessary.
/// However, each pass uses a different parent span (with the individual pass's name), so it's
/// (at best) non-trivial to make that work.
///
/// We could *maybe* use a global parent span called "Pass", with a name field, but that's extremely ugly.
pub(crate) struct PassTracing {
    pub(crate) update_tree: bool,
    pub(crate) anim: bool,
    pub(crate) layout: bool,
    /// Compose is the biggest offender, as it is likely caused by a mouse move.
    pub(crate) compose: bool,
    pub(crate) paint: bool,
    pub(crate) access: bool,
}

impl PassTracing {
    pub(crate) fn from_env() -> Self {
        let env_var = match std::env::var("MASONRY_TRACE_PASSES") {
            Ok(env_var) => env_var,
            // If it's not set, don't show any passes.
            Err(std::env::VarError::NotPresent) => return Self::unit(false),
            Err(std::env::VarError::NotUnicode(value)) => {
                tracing::error!(
                    ?value,
                    "Couldn't parse `MASONRY_TRACE_PASSES` environment variable: Not valid UTF-8",
                );
                return Self::unit(false);
            }
        };
        let env_var = env_var.trim();

        if env_var.eq_ignore_ascii_case("all") {
            return Self::unit(true);
        }
        let mut result = Self::unit(false);
        let mut show_help = false;
        let mut supported_passes = [
            ("update_tree", &mut result.update_tree),
            ("anim", &mut result.anim),
            ("layout", &mut result.layout),
            ("compose", &mut result.compose),
            ("paint", &mut result.paint),
            ("access", &mut result.access),
        ];
        for input_name in env_var.split(',').map(str::trim) {
            if input_name == "all" {
                tracing::warn!(
                    "`MASONRY_TRACE_PASSES=all` cannot be meaningfully combined with other passes"
                );
                return Self::unit(true);
            }
            if let Some((_, value)) = supported_passes
                .iter_mut()
                .find(|(pass_name, _)| pass_name.eq_ignore_ascii_case(input_name))
            {
                if **value {
                    tracing::warn!(
                        pass = input_name,
                        "MASONRY_TRACE_PASSES: Enabled tracing for same pass twice"
                    );
                }
                **value = true;
            } else {
                tracing::warn!(pass = input_name, "MASONRY_TRACE_PASSES: Unknown pass");
                show_help = true;
            }
        }
        if show_help {
            let supported_str = supported_passes
                .iter()
                .map(|(name, _)| name)
                .copied()
                .collect::<Vec<_>>()
                .join(", ");
            tracing::warn!(
                "Supported passes for the `MASONRY_TRACE_PASSES` environment variable are {supported_str}"
            );
        }
        result
    }

    /// A `PassTracing` where all the fields have the same `value`.
    const fn unit(value: bool) -> Self {
        Self {
            update_tree: value,
            anim: value,
            layout: value,
            compose: value,
            paint: value,
            access: value,
        }
    }
}
