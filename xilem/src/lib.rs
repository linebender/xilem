// Copyright 2024 the Xilem Authors
// SPDX-License-Identifier: Apache-2.0

#![allow(clippy::comparison_chain)]
use std::{any::Any, collections::HashMap};

use masonry::{
    app_driver::AppDriver,
    event_loop_runner,
    widget::{RootWidget, WidgetMut},
    Widget, WidgetId, WidgetPod,
};
pub use masonry::{widget::Axis, Color, TextAlignment};
use winit::{
    dpi::LogicalSize,
    error::EventLoopError,
    window::{Window, WindowAttributes},
};

mod any_view;
mod id;
mod sequence;
mod vec_splice;
pub use any_view::{AnyMasonryView, BoxedMasonryView};
pub mod view;
pub use id::ViewId;
pub use sequence::{ElementSplice, ViewSequence};
pub use vec_splice::VecSplice;

pub struct Xilem<State, Logic, View>
where
    View: MasonryView<State>,
{
    root_widget: RootWidget<View::Element>,
    driver: MasonryDriver<State, Logic, View, View::ViewState>,
}

pub struct MasonryDriver<State, Logic, View, ViewState> {
    state: State,
    logic: Logic,
    current_view: View,
    view_cx: ViewCx,
    view_state: ViewState,
}

impl<State, Logic, View> AppDriver for MasonryDriver<State, Logic, View, View::ViewState>
where
    Logic: FnMut(&mut State) -> View,
    View: MasonryView<State>,
{
    fn on_action(
        &mut self,
        ctx: &mut masonry::app_driver::DriverCtx<'_>,
        widget_id: masonry::WidgetId,
        action: masonry::Action,
    ) {
        if let Some(id_path) = self.view_cx.widget_map.get(&widget_id) {
            let message_result = self.current_view.message(
                &mut self.view_state,
                id_path.as_slice(),
                Box::new(action),
                &mut self.state,
            );
            let rebuild = match message_result {
                MessageResult::Action(()) => {
                    // It's not entirely clear what to do here
                    true
                }
                MessageResult::RequestRebuild => true,
                MessageResult::Nop => false,
                MessageResult::Stale(_) => {
                    tracing::info!("Discarding message");
                    false
                }
            };
            if rebuild {
                let next_view = (self.logic)(&mut self.state);
                let mut root = ctx.get_root::<RootWidget<View::Element>>();

                self.view_cx.view_tree_changed = false;
                next_view.rebuild(
                    &mut self.view_state,
                    &mut self.view_cx,
                    &self.current_view,
                    root.get_element(),
                );
                if cfg!(debug_assertions) && !self.view_cx.view_tree_changed {
                    tracing::debug!("Nothing changed as result of action");
                }
                self.current_view = next_view;
            }
        } else {
            eprintln!("Got action {action:?} for unknown widget. Did you forget to use `with_action_widget`?");
        }
    }
}

impl<State, Logic, View> Xilem<State, Logic, View>
where
    Logic: FnMut(&mut State) -> View,
    View: MasonryView<State>,
{
    pub fn new(mut state: State, mut logic: Logic) -> Self {
        let first_view = logic(&mut state);
        let mut view_cx = ViewCx {
            id_path: vec![],
            widget_map: HashMap::new(),
            view_tree_changed: false,
        };
        let (pod, view_state) = first_view.build(&mut view_cx);
        let root_widget = RootWidget::from_pod(pod);
        Xilem {
            driver: MasonryDriver {
                current_view: first_view,
                logic,
                state,
                view_cx,
                view_state,
            },
            root_widget,
        }
    }

    // TODO: Make windows a specific view
    pub fn run_windowed(self, window_title: String) -> Result<(), EventLoopError>
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
        self.run_windowed_in(window_attributes)
    }

    // TODO: Make windows into a custom view
    pub fn run_windowed_in(self, window_attributes: WindowAttributes) -> Result<(), EventLoopError>
    where
        State: 'static,
        Logic: 'static,
        View: 'static,
    {
        event_loop_runner::run(window_attributes, self.root_widget, self.driver)
    }
}
pub trait MasonryView<State, Action = ()>: Send + Sync + 'static {
    type Element: Widget;
    type ViewState;

    fn build(&self, cx: &mut ViewCx) -> (WidgetPod<Self::Element>, Self::ViewState);

    fn rebuild(
        &self,
        view_state: &mut Self::ViewState,
        cx: &mut ViewCx,
        prev: &Self,
        element: WidgetMut<Self::Element>,
    );

    fn message(
        &self,
        view_state: &mut Self::ViewState,
        id_path: &[ViewId],
        message: Box<dyn Any>,
        app_state: &mut State,
    ) -> MessageResult<Action>;
}

pub struct ViewCx {
    /// The map from a widgets id to its position in the View tree.
    ///
    /// This includes only the widgets which might send actions
    /// This is currently never cleaned up
    widget_map: HashMap<WidgetId, Vec<ViewId>>,
    id_path: Vec<ViewId>,
    view_tree_changed: bool,
}

impl ViewCx {
    pub fn mark_changed(&mut self) {
        if cfg!(debug_assertions) {
            self.view_tree_changed = true;
        }
    }

    pub fn with_leaf_action_widget<E: Widget>(
        &mut self,
        f: impl FnOnce(&mut Self) -> WidgetPod<E>,
    ) -> (WidgetPod<E>, ()) {
        (self.with_action_widget(f), ())
    }

    pub fn with_action_widget<E: Widget>(
        &mut self,
        f: impl FnOnce(&mut Self) -> WidgetPod<E>,
    ) -> WidgetPod<E> {
        let value = f(self);
        let id = value.id();
        let path = self.id_path.clone();
        self.widget_map.insert(id, path);
        value
    }

    pub fn with_id<R>(&mut self, id: ViewId, f: impl FnOnce(&mut Self) -> R) -> R {
        self.id_path.push(id);
        let res = f(self);
        self.id_path.pop();
        res
    }
}

/// A result wrapper type for event handlers.
#[derive(Default)]
pub enum MessageResult<A> {
    Action(A),
    RequestRebuild,
    #[default]
    Nop,
    Stale(Box<dyn Any>),
}
