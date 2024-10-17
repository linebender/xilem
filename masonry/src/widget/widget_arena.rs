// Copyright 2024 the Xilem Authors
// SPDX-License-Identifier: Apache-2.0

use crate::tree_arena::{ArenaMut, ArenaRef, TreeArena};
use crate::{Widget, WidgetId, WidgetState};

pub(crate) struct WidgetArena {
    pub(crate) widgets: TreeArena<Box<dyn Widget>>,
    // TODO - Rename to "states"
    pub(crate) widget_states: TreeArena<WidgetState>,
}

impl WidgetArena {
    pub(crate) fn has(&self, widget_id: WidgetId) -> bool {
        self.widgets.find(widget_id.to_raw()).is_some()
    }

    #[track_caller]
    pub(crate) fn parent_of(&self, widget_id: WidgetId) -> Option<WidgetId> {
        let widget_ref = self
            .widgets
            .find(widget_id.to_raw())
            .expect("parent_of: widget not found in arena");

        let id = widget_ref.parent_id?;
        Some(WidgetId(id.try_into().unwrap()))
    }

    #[track_caller]
    pub(crate) fn get_pair(
        &self,
        widget_id: WidgetId,
    ) -> (ArenaRef<Box<dyn Widget>>, ArenaRef<WidgetState>) {
        let widget = self
            .widgets
            .find(widget_id.to_raw())
            .expect("get_pair: widget not in widget tree");
        let state = self
            .widget_states
            .find(widget_id.to_raw())
            .expect("get_pair: widget state not in widget tree");
        (widget, state)
    }

    #[track_caller]
    pub(crate) fn get_pair_mut(
        &mut self,
        widget_id: WidgetId,
    ) -> (ArenaMut<Box<dyn Widget>>, ArenaMut<WidgetState>) {
        let widget = self
            .widgets
            .find_mut(widget_id.to_raw())
            .expect("get_pair_mut: widget not in widget tree");
        let state = self
            .widget_states
            .find_mut(widget_id.to_raw())
            .expect("get_pair_mut: widget state not in widget tree");
        (widget, state)
    }

    #[allow(dead_code)]
    #[track_caller]
    pub(crate) fn get_widget(&self, widget_id: WidgetId) -> ArenaRef<Box<dyn Widget>> {
        self.widgets
            .find(widget_id.to_raw())
            .expect("get_widget: widget not in widget tree")
    }

    #[allow(dead_code)]
    #[track_caller]
    pub(crate) fn get_widget_mut(&mut self, widget_id: WidgetId) -> ArenaMut<Box<dyn Widget>> {
        self.widgets
            .find_mut(widget_id.to_raw())
            .expect("get_widget_mut: widget not in widget tree")
    }

    #[track_caller]
    pub(crate) fn get_state(&mut self, widget_id: WidgetId) -> ArenaRef<WidgetState> {
        self.widget_states
            .find(widget_id.to_raw())
            .expect("get_state: widget state not in widget tree")
    }

    #[track_caller]
    pub(crate) fn get_state_mut(&mut self, widget_id: WidgetId) -> ArenaMut<WidgetState> {
        self.widget_states
            .find_mut(widget_id.to_raw())
            .expect("get_state_mut: widget state not in widget tree")
    }
}
