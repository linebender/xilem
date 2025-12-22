// Copyright 2025 the Xilem Authors
// SPDX-License-Identifier: Apache-2.0

use std::collections::HashMap;
use std::sync::Arc;

use masonry::core::{FromDynWidget, Widget, WidgetId, WidgetMut};

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
            environment: Environment::new(),
        }
    }
}
