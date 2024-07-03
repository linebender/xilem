// Copyright 2024 the Xilem Authors
// SPDX-License-Identifier: Apache-2.0

use std::sync::Arc;

use masonry::{
    app_driver::AppDriver,
    event_loop_runner::{self, EventLoopProxy, MasonryUserEvent},
    widget::RootWidget,
    WidgetId,
};
use xilem_core::{DynMessage, Message, MessageResult, Proxy, ProxyError, ViewId};

use crate::{ViewCtx, WidgetView};

pub struct MasonryDriver<State, Logic, View, ViewState> {
    pub(crate) state: State,
    pub(crate) logic: Logic,
    pub(crate) current_view: View,
    pub(crate) ctx: ViewCtx,
    pub(crate) view_state: ViewState,
    #[allow(unused)]
    // Keep the runtime alive
    // Is it more idiomatic to stick this in the `ViewCtx`?
    pub(crate) runtime: tokio::runtime::Runtime,
}

/// The `WidgetId` which async events should be sent to.
pub const ASYNC_MARKER_WIDGET: WidgetId = WidgetId::reserved(0x1000);

/// The action which should be used for async events
pub fn async_action(path: Arc<[ViewId]>, message: Box<dyn Message>) -> masonry::Action {
    masonry::Action::Other(Box::<MessagePackage>::new((path, message)))
}

type MessagePackage = (Arc<[ViewId]>, DynMessage);

impl Proxy for MasonryProxy {
    fn send_message(&self, path: Arc<[ViewId]>, message: DynMessage) -> Result<(), ProxyError> {
        match self
            .0
            .send_event(event_loop_runner::MasonryUserEvent::Action(
                async_action(path, message),
                ASYNC_MARKER_WIDGET,
            )) {
            Ok(()) => Ok(()),
            Err(err) => {
                let MasonryUserEvent::Action(masonry::Action::Other(res), _) = err.0 else {
                    unreachable!(
                        "We know this is the value we just created, which matches this pattern"
                    )
                };
                Err(ProxyError::DriverFinished(
                    res.downcast::<MessagePackage>().unwrap().1,
                ))
            }
        }
    }
}

pub(crate) struct MasonryProxy(pub(crate) EventLoopProxy);

impl<State, Logic, View> AppDriver for MasonryDriver<State, Logic, View, View::ViewState>
where
    Logic: FnMut(&mut State) -> View,
    View: WidgetView<State>,
{
    fn on_action(
        &mut self,
        masonry_ctx: &mut masonry::app_driver::DriverCtx<'_>,
        widget_id: masonry::WidgetId,
        action: masonry::Action,
    ) {
        let message_result = if widget_id == ASYNC_MARKER_WIDGET {
            let masonry::Action::Other(action) = action else {
                panic!();
            };
            let (path, message) = *action.downcast::<MessagePackage>().unwrap();
            // Handle an async path
            self.current_view
                .message(&mut self.view_state, &path, message, &mut self.state)
        } else if let Some(id_path) = self.ctx.widget_map.get(&widget_id) {
            self.current_view.message(
                &mut self.view_state,
                id_path.as_slice(),
                Box::new(action),
                &mut self.state,
            )
        } else {
            eprintln!("Got action {action:?} for unknown widget. Did you forget to use `with_action_widget`?");
            return;
        };
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

            let mut root = masonry_ctx.get_root::<RootWidget<View::Widget>>();

            next_view.rebuild(
                &self.current_view,
                &mut self.view_state,
                &mut self.ctx,
                root.get_element(),
            );
            if cfg!(debug_assertions) && !self.ctx.view_tree_changed {
                tracing::debug!("Nothing changed as result of action");
            }
            self.current_view = next_view;
        }
    }
}
