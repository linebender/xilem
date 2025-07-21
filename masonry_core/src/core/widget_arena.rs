// Copyright 2024 the Xilem Authors
// SPDX-License-Identifier: Apache-2.0

use tree_arena::{ArenaMut, ArenaMutList, ArenaRef, ArenaRefList, TreeArena};

use crate::core::{Widget, WidgetId, WidgetState};
use crate::util::AnyMap;

pub(crate) struct WidgetArena {
    pub(crate) widgets: TreeArena<Box<dyn Widget>>,
    pub(crate) states: TreeArena<WidgetState>,
    pub(crate) properties: TreeArena<AnyMap>,
}

#[derive(Clone, Copy)]
pub(crate) struct WidgetArenaRef<'a> {
    pub(crate) widget_state_children: ArenaRefList<'a, WidgetState>,
    pub(crate) widget_children: ArenaRefList<'a, Box<dyn Widget>>,
    pub(crate) properties_children: ArenaRefList<'a, AnyMap>,
}

pub(crate) struct WidgetArenaMut<'a> {
    pub(crate) widget_state_children: ArenaMutList<'a, WidgetState>,
    pub(crate) widget_children: ArenaMutList<'a, Box<dyn Widget>>,
    pub(crate) properties_children: ArenaMutList<'a, AnyMap>,
}

impl WidgetArena {
    pub(crate) fn has(&self, widget_id: WidgetId) -> bool {
        self.widgets.find(widget_id).is_some()
    }

    #[track_caller]
    pub(crate) fn parent_of(&self, widget_id: WidgetId) -> Option<WidgetId> {
        let widget_ref = self
            .widgets
            .find(widget_id)
            .expect("parent_of: widget not found in arena");

        let id = widget_ref.parent_id?;
        Some(WidgetId(id.try_into().unwrap()))
    }

    #[track_caller]
    pub(crate) fn get_all(
        &self,
        widget_id: WidgetId,
    ) -> (
        ArenaRef<'_, Box<dyn Widget>>,
        ArenaRef<'_, WidgetState>,
        ArenaRef<'_, AnyMap>,
    ) {
        let widget = self
            .widgets
            .find(widget_id)
            .expect("get_pair: widget not in widget tree");
        let state = self
            .states
            .find(widget_id)
            .expect("get_pair: widget state not in widget tree");
        let properties = self
            .properties
            .find(widget_id)
            .expect("get_pair: widget properties not in widget tree");
        (widget, state, properties)
    }

    #[track_caller]
    pub(crate) fn get_all_mut(
        &mut self,
        widget_id: WidgetId,
    ) -> (
        ArenaMut<'_, Box<dyn Widget>>,
        ArenaMut<'_, WidgetState>,
        ArenaMut<'_, AnyMap>,
    ) {
        let widget = self
            .widgets
            .find_mut(widget_id)
            .expect("get_pair_mut: widget not in widget tree");
        let state = self
            .states
            .find_mut(widget_id)
            .expect("get_pair_mut: widget state not in widget tree");
        let properties = self
            .properties
            .find_mut(widget_id)
            .expect("get_pair_mut: widget properties not in widget tree");
        (widget, state, properties)
    }

    #[allow(dead_code, reason = "might be useful later")]
    #[track_caller]
    pub(crate) fn get_widget(&self, widget_id: WidgetId) -> ArenaRef<'_, Box<dyn Widget>> {
        self.widgets
            .find(widget_id)
            .expect("get_widget: widget not in widget tree")
    }

    #[allow(dead_code, reason = "might be useful later")]
    #[track_caller]
    pub(crate) fn get_widget_mut(&mut self, widget_id: WidgetId) -> ArenaMut<'_, Box<dyn Widget>> {
        self.widgets
            .find_mut(widget_id)
            .expect("get_widget_mut: widget not in widget tree")
    }

    #[track_caller]
    pub(crate) fn get_state(&mut self, widget_id: WidgetId) -> ArenaRef<'_, WidgetState> {
        self.states
            .find(widget_id)
            .expect("get_state: widget state not in widget tree")
    }

    #[track_caller]
    pub(crate) fn get_state_mut(&mut self, widget_id: WidgetId) -> ArenaMut<'_, WidgetState> {
        self.states
            .find_mut(widget_id)
            .expect("get_state_mut: widget state not in widget tree")
    }
}

impl<'a> WidgetArenaMut<'a> {
    #[allow(unused, reason = "May be used later")]
    pub(crate) fn reborrow(&self) -> WidgetArenaRef<'_> {
        WidgetArenaRef {
            widget_state_children: self.widget_state_children.reborrow(),
            widget_children: self.widget_children.reborrow(),
            properties_children: self.properties_children.reborrow(),
        }
    }

    pub(crate) fn reborrow_mut(&mut self) -> WidgetArenaMut<'_> {
        WidgetArenaMut {
            widget_state_children: self.widget_state_children.reborrow_mut(),
            widget_children: self.widget_children.reborrow_mut(),
            properties_children: self.properties_children.reborrow_mut(),
        }
    }
}
