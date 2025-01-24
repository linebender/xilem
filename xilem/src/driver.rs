// Copyright 2024 the Xilem Authors
// SPDX-License-Identifier: Apache-2.0

use std::sync::Arc;

use masonry::app::{AppDriver, EventLoopProxy, MasonryState, MasonryUserEvent};
use masonry::core::WidgetId;
use masonry::widgets::RootWidget;

use crate::core::{DynMessage, Message, MessageResult, ProxyError, RawProxy, ViewId};
use crate::{ViewCtx, WidgetView};

pub struct MasonryDriver<State, Logic, View, ViewState> {
    pub(crate) state: State,
    pub(crate) logic: Logic,
    pub(crate) current_view: View,
    pub(crate) ctx: ViewCtx,
    pub(crate) view_state: ViewState,
    // Fonts which will be registered on startup.
    pub(crate) fonts: Vec<Vec<u8>>,
}

/// The `WidgetId` which async events should be sent to.
pub const ASYNC_MARKER_WIDGET: WidgetId = WidgetId::reserved(0x1000);

/// The action which should be used for async events.
pub fn async_action(path: Arc<[ViewId]>, message: Box<dyn Message>) -> masonry::core::Action {
    masonry::core::Action::Other(Box::<MessagePackage>::new((path, message)))
}

/// The type used to send a message for async events.
type MessagePackage = (Arc<[ViewId]>, DynMessage);

impl RawProxy for MasonryProxy {
    fn send_message(&self, path: Arc<[ViewId]>, message: DynMessage) -> Result<(), ProxyError> {
        match self.0.send_event(MasonryUserEvent::Action(
            async_action(path, message),
            ASYNC_MARKER_WIDGET,
        )) {
            Ok(()) => Ok(()),
            Err(err) => {
                let MasonryUserEvent::Action(masonry::core::Action::Other(res), _) = err.0 else {
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
    fn dyn_debug(&self) -> &dyn std::fmt::Debug {
        self
    }
}

#[derive(Debug)]
pub struct MasonryProxy(pub(crate) EventLoopProxy);

impl MasonryProxy {
    pub fn new(proxy: EventLoopProxy) -> Self {
        Self(proxy)
    }
}

impl<State, Logic, View> AppDriver for MasonryDriver<State, Logic, View, View::ViewState>
where
    Logic: FnMut(&mut State) -> View,
    View: WidgetView<State>,
{
    fn on_action(
        &mut self,
        masonry_ctx: &mut masonry::app::DriverCtx<'_>,
        widget_id: WidgetId,
        action: masonry::core::Action,
    ) {
        let message_result = if widget_id == ASYNC_MARKER_WIDGET {
            let masonry::core::Action::Other(action) = action else {
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
            tracing::error!("Got action {action:?} for unknown widget. Did you forget to use `with_action_widget`?");
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

            masonry_ctx.render_root().edit_root_widget(|mut root| {
                let mut root = root.downcast::<RootWidget<View::Widget>>();
                next_view.rebuild(
                    &self.current_view,
                    &mut self.view_state,
                    &mut self.ctx,
                    RootWidget::child_mut(&mut root),
                );
                self.current_view = next_view;
            });
        }
        if cfg!(debug_assertions) && rebuild && !masonry_ctx.content_changed() {
            tracing::debug!("Nothing changed as result of action");
        }
    }
    fn on_start(&mut self, state: &mut MasonryState) {
        let root = state.get_root();
        // Register all provided fonts
        // self.fonts is never used again, so we may as well deallocate it.
        for font in std::mem::take(&mut self.fonts).drain(..) {
            // We currently don't do anything with the resulting family information,
            // because we don't have an easy way to return this to the application.
            drop(root.register_fonts(font));
        }
    }
}
