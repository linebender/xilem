// Copyright 2024 the Xilem Authors
// SPDX-License-Identifier: Apache-2.0

#![allow(clippy::comparison_chain)]
use std::collections::HashMap;

use driver::MasonryDriver;
use masonry::{
    dpi::LogicalSize,
    event_loop_runner,
    widget::{RootWidget, WidgetMut},
    Widget, WidgetId, WidgetPod,
};
use winit::{
    error::EventLoopError,
    window::{Window, WindowAttributes},
};
use xilem_core::{MessageResult, SuperElement, View, ViewElement, ViewId, ViewPathTracker};

pub use masonry::{
    dpi,
    event_loop_runner::{EventLoop, EventLoopBuilder},
    widget::Axis,
    Color, TextAlignment,
};
pub use xilem_core as core;

mod any_view;
pub use any_view::AnyWidgetView;
mod driver;
pub use driver::{AppToXilemInterface, XilemToAppInterface};
pub mod view;

pub struct Xilem<State, Logic, View>
where
    View: WidgetView<State>,
{
    root_widget: RootWidget<View::Widget>,
    driver: MasonryDriver<State, Logic, View, View::ViewState>,
}

impl<State, Logic, View> Xilem<State, Logic, View>
where
    Logic: FnMut(&mut State) -> View,
    View: WidgetView<State>,
{
    pub fn new(mut state: State, mut logic: Logic) -> Self {
        let first_view = logic(&mut state);
        let mut view_ctx = ViewCtx::default();
        let (pod, view_state) = first_view.build(&mut view_ctx);
        let root_widget = RootWidget::from_pod(pod.inner);
        Xilem {
            driver: MasonryDriver {
                current_view: first_view,
                logic,
                state,
                view_ctx,
                view_state,
                app_interface: None,
            },
            root_widget,
        }
    }

    pub fn with_app_interface(mut self, app_interface: Box<dyn XilemToAppInterface<State>>) -> Self {
        self.driver.app_interface = Some(app_interface);
        self
    }

    // TODO: Make windows a specific view
    pub fn run_windowed(
        self,
        // We pass in the event loop builder to allow
        // This might need to be generic over the event type?
        event_loop: EventLoopBuilder,
        window_title: String,
    ) -> Result<(), EventLoopError>
    where
        State: 'static,
        Logic: 'static,
        View: 'static,
    {
        let window_size = LogicalSize::new(600., 800.);
        let window_attributes = Window::default_attributes()
            .with_title(window_title)
            .with_resizable(true)
            .with_min_inner_size(window_size);
        self.run_windowed_in(event_loop, window_attributes)
    }

    // TODO: Make windows into a custom view
    pub fn run_windowed_in(
        self,
        event_loop: EventLoopBuilder,
        window_attributes: WindowAttributes,
    ) -> Result<(), EventLoopError>
    where
        State: 'static,
        Logic: 'static,
        View: 'static,
    {
        event_loop_runner::run(event_loop, window_attributes, self.root_widget, self.driver)
    }
}

/// A container for a [Masonry](masonry) widget to be used with Xilem.
///
/// Equivalent to [`WidgetPod<W>`], but in the [`xilem`](crate) crate to work around the orphan rule.
pub struct Pod<W: Widget> {
    pub inner: WidgetPod<W>,
}

impl<W: Widget> Pod<W> {
    /// Create a new `Pod` for `inner`.
    pub fn new(inner: W) -> Self {
        Self::from(WidgetPod::new(inner))
    }
}

impl<W: Widget> From<WidgetPod<W>> for Pod<W> {
    fn from(inner: WidgetPod<W>) -> Self {
        Pod { inner }
    }
}

impl<W: Widget> ViewElement for Pod<W> {
    type Mut<'a> = WidgetMut<'a, W>;
}

impl<W: Widget> SuperElement<Pod<W>> for Pod<Box<dyn Widget>> {
    fn upcast(child: Pod<W>) -> Self {
        child.inner.boxed().into()
    }

    fn with_downcast_val<R>(
        mut this: Self::Mut<'_>,
        f: impl FnOnce(<Pod<W> as xilem_core::ViewElement>::Mut<'_>) -> R,
    ) -> (Self::Mut<'_>, R) {
        let downcast = this.downcast();
        let ret = f(downcast);
        (this, ret)
    }
}

pub trait WidgetView<State, Action = ()>:
    View<State, Action, ViewCtx, Element = Pod<Self::Widget>> + Send + Sync
{
    type Widget: Widget;
}

impl<V, State, Action, W> WidgetView<State, Action> for V
where
    V: View<State, Action, ViewCtx, Element = Pod<W>> + Send + Sync,
    W: Widget,
{
    type Widget = W;
}

#[derive(Default)]
pub struct ViewCtx {
    /// The map from a widgets id to its position in the View tree.
    ///
    /// This includes only the widgets which might send actions
    /// This is currently never cleaned up
    widget_map: HashMap<WidgetId, Vec<ViewId>>,
    id_path: Vec<ViewId>,
    view_tree_changed: bool,
}

impl ViewPathTracker for ViewCtx {
    fn push_id(&mut self, id: xilem_core::ViewId) {
        self.id_path.push(id);
    }

    fn pop_id(&mut self) {
        self.id_path.pop();
    }

    fn view_path(&mut self) -> &[xilem_core::ViewId] {
        &self.id_path
    }
}

impl ViewCtx {
    pub fn mark_changed(&mut self) {
        if cfg!(debug_assertions) {
            self.view_tree_changed = true;
        }
    }

    pub fn with_leaf_action_widget<E: Widget>(
        &mut self,
        f: impl FnOnce(&mut Self) -> Pod<E>,
    ) -> (Pod<E>, ()) {
        (self.with_action_widget(f), ())
    }

    pub fn with_action_widget<E: Widget>(&mut self, f: impl FnOnce(&mut Self) -> Pod<E>) -> Pod<E> {
        let value = f(self);
        let id = value.inner.id();
        let path = self.id_path.clone();
        self.widget_map.insert(id, path);
        value
    }

    pub fn teardown_leaf<E: Widget>(&mut self, widget: WidgetMut<E>) {
        self.widget_map.remove(&widget.ctx.widget_id());
    }
}
