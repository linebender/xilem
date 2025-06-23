// Copyright 2024 the Xilem Authors
// SPDX-License-Identifier: Apache-2.0

use tree_arena::{ArenaMut, ArenaMutList, ArenaRef, TreeArena};

use crate::core::{Widget, WidgetId, WidgetState};
use crate::util::AnyMap;

pub(crate) struct WidgetArena {
    pub(crate) widgets: TreeArena<Box<dyn Widget>>,
    pub(crate) states: TreeArena<WidgetState>,
    pub(crate) properties: TreeArena<AnyMap>,
}

pub(crate) struct WidgetItemMut<'a> {
    pub(crate) widget: &'a mut Box<dyn Widget>,
    pub(crate) state: &'a mut WidgetState,
    pub(crate) properties: &'a mut AnyMap,
}

pub(crate) struct WidgetArenaMut<'a> {
    pub(crate) widget_children: ArenaMutList<'a, Box<dyn Widget>>,
    pub(crate) state_children: ArenaMutList<'a, WidgetState>,
    pub(crate) properties_children: ArenaMutList<'a, AnyMap>,
}

impl WidgetArena {
    pub(crate) fn roots_mut(&mut self) -> WidgetArenaMut<'_> {
        WidgetArenaMut {
            state_children: self.states.roots_mut(),
            widget_children: self.widgets.roots_mut(),
            properties_children: self.properties.roots_mut(),
        }
    }

    pub(crate) fn get_mut(&mut self, id: WidgetId) -> (WidgetItemMut<'_>, WidgetArenaMut<'_>) {
        let widget = self
            .widgets
            .find_mut(id)
            .expect("get_mut: widget not in widget tree");
        let widget_state = self
            .states
            .find_mut(id)
            .expect("get_mut: widget state not in widget tree");
        let properties = self
            .properties
            .find_mut(id)
            .expect("get_mut: widget properties not in widget tree");
        (
            WidgetItemMut {
                widget: widget.item,
                state: widget_state.item,
                properties: properties.item,
            },
            WidgetArenaMut {
                widget_children: widget.children,
                state_children: widget_state.children,
                properties_children: properties.children,
            },
        )
    }

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

    #[allow(dead_code)]
    #[track_caller]
    pub(crate) fn get_widget(&self, widget_id: WidgetId) -> ArenaRef<'_, Box<dyn Widget>> {
        self.widgets
            .find(widget_id)
            .expect("get_widget: widget not in widget tree")
    }

    #[allow(dead_code)]
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

impl WidgetArenaMut<'_> {
    pub(crate) fn child_mut<'c>(
        &'c mut self,
        child_id: WidgetId,
    ) -> Option<(WidgetItemMut<'c>, WidgetArenaMut<'c>)> {
        let widget = self.widget_children.item_mut(child_id)?;
        let state = self.state_children.item_mut(child_id)?;
        let properties = self.properties_children.item_mut(child_id)?;

        Some((
            WidgetItemMut {
                widget: widget.item,
                state: state.item,
                properties: properties.item,
            },
            WidgetArenaMut {
                widget_children: widget.children,
                state_children: state.children,
                properties_children: properties.children,
            },
        ))
    }

    pub(crate) fn reborrow_mut(&mut self) -> WidgetArenaMut<'_> {
        WidgetArenaMut {
            widget_children: self.widget_children.reborrow_mut(),
            state_children: self.state_children.reborrow_mut(),
            properties_children: self.properties_children.reborrow_mut(),
        }
    }
}

impl WidgetItemMut<'_> {
    pub(crate) fn reborrow_mut(&mut self) -> WidgetItemMut<'_> {
        WidgetItemMut {
            widget: &mut *self.widget,
            state: &mut *self.state,
            properties: &mut *self.properties,
        }
    }
}
