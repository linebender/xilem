// Copyright 2024 the Xilem Authors
// SPDX-License-Identifier: Apache-2.0

use tree_arena::{ArenaMut, ArenaRef, TreeArena};

use crate::core::{PropertySet, Widget, WidgetId, WidgetState};
use crate::util::TypeSet;

pub(crate) struct WidgetArena {
    pub(crate) nodes: TreeArena<WidgetArenaNode>,
}

pub(crate) struct WidgetArenaNode {
    pub(crate) widget: Box<dyn Widget>,
    pub(crate) state: WidgetState,
    pub(crate) properties: PropertySet,
    pub(crate) changed_properties: TypeSet,
}

impl WidgetArena {
    pub(crate) fn has(&self, widget_id: WidgetId) -> bool {
        self.nodes.find(widget_id).is_some()
    }

    #[track_caller]
    pub(crate) fn parent_of(&self, widget_id: WidgetId) -> Option<WidgetId> {
        let widget_ref = self
            .nodes
            .find(widget_id)
            .expect("parent_of: widget not found in arena");

        let id = widget_ref.parent_id?;
        Some(WidgetId(id.try_into().unwrap()))
    }

    #[track_caller]
    pub(crate) fn get_node(&self, widget_id: WidgetId) -> ArenaRef<'_, WidgetArenaNode> {
        self.nodes
            .find(widget_id)
            .expect("get_pair: widget not in widget tree")
    }

    #[track_caller]
    pub(crate) fn get_node_mut(&mut self, widget_id: WidgetId) -> ArenaMut<'_, WidgetArenaNode> {
        self.nodes
            .find_mut(widget_id)
            .expect("get_pair_mut: widget not in widget tree")
    }

    #[track_caller]
    pub(crate) fn get_state(&mut self, widget_id: WidgetId) -> &WidgetState {
        &self
            .nodes
            .find(widget_id)
            .expect("get_state: widget state not in widget tree")
            .item
            .state
    }

    #[track_caller]
    pub(crate) fn get_state_mut(&mut self, widget_id: WidgetId) -> &mut WidgetState {
        &mut self
            .nodes
            .find_mut(widget_id)
            .expect("get_state_mut: widget state not in widget tree")
            .item
            .state
    }
}
