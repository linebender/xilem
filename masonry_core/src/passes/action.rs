// Copyright 2026 the Xilem Authors
// SPDX-License-Identifier: Apache-2.0

use tracing::warn;

use crate::app::{RenderRoot, RenderRootSignal};
use crate::core::{ActionCtx, ErasedAction, Handled, PropertiesMut, WidgetId};
use crate::passes::{enter_span, merge_state_up};

/// Propagates the `action` from the `source` all the way up to the root widget.
fn handle_action(root: &mut RenderRoot, action: &ErasedAction, source: WidgetId) -> Handled {
    let mut next_widget_id = root.widget_arena.parent_of(source);
    let mut is_handled = false;

    while let Some(widget_id) = next_widget_id {
        next_widget_id = root.widget_arena.parent_of(widget_id);
        let mut node = root.widget_arena.get_node_mut(widget_id);

        // Even disabled widgets take part in action propagation.
        // This keeps the action handling logic consistent.

        if !is_handled {
            let _span = enter_span(&node.item.state);
            let mut ctx = ActionCtx {
                global_state: &mut root.global_state,
                widget_state: &mut node.item.state,
                children: node.children.reborrow_mut(),
                default_properties: &root.default_properties,
                is_handled: false,
            };
            let widget = &mut *node.item.widget;

            let mut props = PropertiesMut {
                map: &mut node.item.properties,
                default_map: root.default_properties.for_widget(widget.type_id()),
            };
            widget.on_action(&mut ctx, &mut props, action, source);
            is_handled = ctx.is_handled;
        }

        merge_state_up(&mut root.widget_arena, widget_id);
    }

    Handled::from(is_handled)
}

/// Propagate actions from the source widgets up all the way to the app driver.
///
/// See the [passes documentation](crate::doc::pass_system#the-action-pass).
pub(crate) fn run_action_pass(root: &mut RenderRoot) {
    let actions = std::mem::take(&mut root.global_state.actions);
    for (action, source) in actions {
        if !root.has_widget(source) {
            // The widget was removed from the tree before its ancestors could handle its action.
            // The action should be dropped and not propagated at all. Partial propagation, e.g.
            // widget tree bubbling being skipped and the action going straight to the app driver,
            // is not safe. That is because an action not happening at all is easy to reason about.
            // While partially processed actions can lead to incredibly hard to debug subtle issues
            // For example if a container widget monitors for actions and generates its own
            // companion action for the app driver. Then the app driver expecting the companion
            // action to always follow the initial action. Partial propagation would break that
            // assumption in the rare cases where the initial action source has just been deleted.
            warn!(
                "Aborting action {} propagation because the source widget {source} \
				has been removed from the tree.",
                action.type_name(),
            );
            continue;
        }
        if let Handled::No = handle_action(root, &action, source) {
            root.global_state
                .emit_signal(RenderRootSignal::Action(action, source));
        }
    }
}
