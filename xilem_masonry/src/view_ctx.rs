// Copyright 2025 the Xilem Authors
// SPDX-License-Identifier: Apache-2.0

use std::any::TypeId;
use std::collections::{HashMap, HashSet};
use std::sync::Arc;

use masonry::core::{FromDynWidget, Property, Widget, WidgetId, WidgetMut};

use crate::Pod;
use crate::core::{Environment, RawProxy, ViewId, ViewPathTracker};

/// A context type passed to various methods of Xilem traits.
pub struct ViewCtx {
    /// The map from a widgets id to its position in the View tree.
    ///
    /// This includes only the widgets which might send actions
    widget_map: HashMap<WidgetId, Vec<ViewId>>,
    id_path: Vec<ViewId>,
    proxy: Arc<dyn RawProxy>,
    runtime: Arc<tokio::runtime::Runtime>,
    props_changed: HashSet<(WidgetId, TypeId)>,
    environment: Environment,
}

impl ViewPathTracker for ViewCtx {
    fn push_id(&mut self, id: ViewId) {
        self.id_path.push(id);
    }

    fn pop_id(&mut self) {
        self.id_path.pop();
    }

    fn view_path(&mut self) -> &[ViewId] {
        &self.id_path
    }

    fn environment(&mut self) -> &mut Environment {
        &mut self.environment
    }
}

impl ViewCtx {
    /// Returns the list of ids of [`WidgetView`](crate::WidgetView)s that must be traversed to get the widget with the given id.
    ///
    /// Only applies to widgets on which [`Self::record_action_source`] was called.
    pub fn get_id_path(&self, widget_id: WidgetId) -> Option<&Vec<ViewId>> {
        self.widget_map.get(&widget_id)
    }

    // TODO - Remove?
    /// Wrapper around [`Pod::new()`].
    pub fn create_pod<W: Widget + FromDynWidget>(&mut self, widget: W) -> Pod<W> {
        Pod::new(widget)
    }

    /// Helper method which passes through the returned `Pod` and passes its id to [`Self::record_action_source`].
    pub fn with_action_widget<W: Widget + FromDynWidget + ?Sized>(
        &mut self,
        f: impl FnOnce(&mut Self) -> Pod<W>,
    ) -> Pod<W> {
        let value = f(self);
        self.record_action_source(value.new_widget.id());
        value
    }

    /// Records that the actions from the widget `id` should be routed to this view.
    pub fn record_action_source(&mut self, id: WidgetId) {
        let path = self.id_path.clone();
        self.widget_map.insert(id, path);
    }

    /// Removes this widget's id path from the routing map.
    pub fn teardown_action_source<W: Widget + FromDynWidget + ?Sized>(
        &mut self,
        widget: WidgetMut<'_, W>,
    ) {
        self.widget_map.remove(&widget.ctx.widget_id());
    }

    /// Returns a reference to the app's tokio runtime.
    pub fn runtime(&self) -> &tokio::runtime::Runtime {
        &self.runtime
    }

    /// Marks the property `P` of the widget with the given id as changed.
    ///
    /// This is used to avoid bugs when multiple `Prop` views are stacked on the same widget.
    ///
    /// This should be reset at the end of each rebuild.
    pub fn mark_prop_changed<P: Property>(&mut self, id: WidgetId) {
        self.props_changed.insert((id, TypeId::of::<P>()));
    }

    /// Checks if the property `P` of the widget with the given id has changed during this rebuild.
    ///
    /// This is used to avoid bugs when multiple `Prop` views are stacked on the same widget.
    pub fn prop_has_changed<P: Property>(&self, id: WidgetId) -> bool {
        self.props_changed.contains(&(id, TypeId::of::<P>()))
    }

    /// Resets the changed properties for all widgets.
    ///
    /// This should be called at the start of each rebuild.
    pub fn reset_changed_props(&mut self) {
        self.props_changed.clear();
    }

    /// Returns an event queue to which [`SendMessage`](crate::core::SendMessage)s can be submitted.
    pub fn proxy(&self) -> Arc<dyn RawProxy + 'static> {
        self.proxy.clone()
    }

    /// Creates a new `ViewCtx` for rebuilding the widget tree.
    ///
    /// You almost never need to call this method unless you're building your own framework.
    pub fn new(proxy: Arc<dyn RawProxy>, runtime: Arc<tokio::runtime::Runtime>) -> Self {
        Self {
            widget_map: HashMap::default(),
            id_path: Vec::new(),
            proxy,
            runtime,
            props_changed: HashSet::default(),
            environment: Environment::new(),
        }
    }
}
