// Copyright 2024 the Xilem Authors
// SPDX-License-Identifier: Apache-2.0

use tracing::span::EnteredSpan;

use crate::render_root::RenderRootState;
use crate::tree_arena::{ArenaMut, ArenaMutChildren, ArenaRef};
use crate::widget::WidgetArena;
use crate::{QueryCtx, Widget, WidgetId, WidgetState};

pub(crate) mod accessibility;
pub(crate) mod anim;
pub(crate) mod compose;
pub(crate) mod event;
pub(crate) mod layout;
pub(crate) mod mutate;
pub(crate) mod paint;
pub(crate) mod update;

#[must_use = "Span will be immediately closed if dropped"]
pub(crate) fn enter_span_if(
    enabled: bool,
    global_state: &RenderRootState,
    widget: ArenaRef<'_, Box<dyn Widget>>,
    state: ArenaRef<'_, WidgetState>,
) -> Option<EnteredSpan> {
    if enabled {
        Some(enter_span(global_state, widget, state))
    } else {
        None
    }
}

#[must_use = "Span will be immediately closed if dropped"]
pub(crate) fn enter_span(
    global_state: &RenderRootState,
    widget: ArenaRef<'_, Box<dyn Widget>>,
    state: ArenaRef<'_, WidgetState>,
) -> EnteredSpan {
    let ctx = QueryCtx {
        global_state,
        widget_state: state.item,
        widget_state_children: state.children,
        widget_children: widget.children,
    };
    widget.item.make_trace_span(&ctx).entered()
}

pub(crate) fn recurse_on_children(
    id: WidgetId,
    mut widget: ArenaMut<'_, Box<dyn Widget>>,
    mut state: ArenaMutChildren<'_, WidgetState>,
    mut callback: impl FnMut(ArenaMut<'_, Box<dyn Widget>>, ArenaMut<'_, WidgetState>),
) {
    let parent_name = widget.item.short_type_name();
    let parent_id = id;

    for child_id in widget.item.children_ids() {
        let widget = widget.children.get_child_mut(child_id).unwrap_or_else(|| {
            panic!(
                "Error in '{}' #{}: cannot find child #{} returned by children_ids()",
                parent_name, parent_id, child_id
            )
        });
        let state = state.get_child_mut(child_id).unwrap_or_else(|| {
            panic!(
                "Error in '{}' #{}: cannot find child #{} returned by children_ids()",
                parent_name, parent_id, child_id
            )
        });

        callback(widget, state);
    }
}

pub(crate) fn merge_state_up(arena: &mut WidgetArena, widget_id: WidgetId) {
    let parent_id = arena.parent_of(widget_id);

    let Some(parent_id) = parent_id else {
        // We've reached the root
        return;
    };

    let mut parent_state_mut = arena.widget_states.find_mut(parent_id).unwrap();
    let child_state_mut = parent_state_mut.children.get_child_mut(widget_id).unwrap();

    parent_state_mut.item.merge_up(child_state_mut.item);
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
