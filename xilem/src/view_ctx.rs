// Copyright 2025 the Xilem Authors
// SPDX-License-Identifier: Apache-2.0

use std::collections::HashMap;
use std::sync::Arc;

use masonry::core::{FromDynWidget, Widget, WidgetId, WidgetMut};

use crate::Pod;
use crate::core::{AsyncCtx, Environment, RawProxy, ViewId, ViewPathTracker};

/// A context type passed to various methods of Xilem traits.
pub struct ViewCtx {
    /// The map from a widgets id to its position in the View tree.
    ///
    /// This includes only the widgets which might send actions
    widget_map: HashMap<WidgetId, Vec<ViewId>>,
    id_path: Vec<ViewId>,
    proxy: Arc<dyn RawProxy>,
    runtime: Arc<tokio::runtime::Runtime>,
    state_changed: bool,
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

// Private items
impl ViewCtx {
    pub(crate) fn new(proxy: Arc<dyn RawProxy>, runtime: Arc<tokio::runtime::Runtime>) -> Self {
        Self {
            widget_map: HashMap::default(),
            id_path: Vec::new(),
            proxy,
            runtime,
            state_changed: true,
            environment: Environment::new(),
        }
    }

    pub(crate) fn set_state_changed(&mut self, value: bool) {
        self.state_changed = value;
    }

    pub(crate) fn get_id_path(&self, widget_id: WidgetId) -> Option<&Vec<ViewId>> {
        self.widget_map.get(&widget_id)
    }
}

#[expect(missing_docs, reason = "TODO - Document these items")]
impl ViewCtx {
    pub fn create_pod<W: Widget + FromDynWidget>(&mut self, widget: W) -> Pod<W> {
        Pod::new(widget)
    }

    pub fn with_leaf_action_widget<W: Widget + FromDynWidget + ?Sized>(
        &mut self,
        f: impl FnOnce(&mut Self) -> Pod<W>,
    ) -> (Pod<W>, ()) {
        (self.with_action_widget(f), ())
    }

    pub fn with_action_widget<W: Widget + FromDynWidget + ?Sized>(
        &mut self,
        f: impl FnOnce(&mut Self) -> Pod<W>,
    ) -> Pod<W> {
        let value = f(self);
        self.record_action(value.id);
        value
    }

    /// Record that the actions from the widget `id` should be routed to this view.
    pub fn record_action(&mut self, id: WidgetId) {
        let path = self.id_path.clone();
        self.widget_map.insert(id, path);
    }

    /// Whether the app's state changed since the last rebuild.
    ///
    /// This is useful for views whose current value depends on current app state.
    /// (That is, currently only virtual scrolling)
    pub fn state_changed(&self) -> bool {
        self.state_changed
    }

    pub fn teardown_leaf<W: Widget + FromDynWidget + ?Sized>(&mut self, widget: WidgetMut<'_, W>) {
        self.widget_map.remove(&widget.ctx.widget_id());
    }

    pub fn runtime(&self) -> &tokio::runtime::Runtime {
        &self.runtime
    }

    pub fn proxy(&self) -> Arc<dyn RawProxy + 'static> {
        self.proxy.clone()
    }
}

impl AsyncCtx for ViewCtx {
    fn proxy(&mut self) -> Arc<dyn RawProxy> {
        self.proxy.clone()
    }
}
