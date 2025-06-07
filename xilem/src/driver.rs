// Copyright 2024 the Xilem Authors
// SPDX-License-Identifier: Apache-2.0

#![expect(missing_docs, reason = "TODO - Document these items")]

use std::fmt::Debug;
use std::sync::Arc;

use masonry_winit::app::{AppDriver, MasonryState, MasonryUserEvent, WindowId};
use masonry_winit::core::{Widget, WidgetId};
use masonry_winit::peniko::Blob;
use masonry_winit::widgets::RootWidget;

use crate::core::{DynMessage, MessageResult, ProxyError, RawProxy, ViewId};
use crate::{ViewCtx, WidgetMap, WidgetView};

pub struct MasonryDriver<State, Logic, View, ViewState> {
    state: State,
    logic: Logic,
    current_view: View,
    ctx: ViewCtx,
    view_state: ViewState,
    window_id: WindowId,
    // Fonts which will be registered on startup.
    fonts: Vec<Blob<u8>>,
}

impl<State, Logic, View, ViewState> MasonryDriver<State, Logic, View, ViewState>
where
    Logic: FnMut(&mut State) -> View,
    View: WidgetView<State, ViewState = ViewState>,
{
    pub(crate) fn new(
        mut state: State,
        mut logic: Logic,
        // TODO: narrow down MasonryUserEvent in event_sink once masonry_winit supports custom event types
        // (we only ever use it to send MasonryUserEvent::Action with ASYNC_MARKER_WIDGET)
        event_sink: impl Fn(MasonryUserEvent) -> Result<(), MasonryUserEvent> + Send + Sync + 'static,
        runtime: tokio::runtime::Runtime,
        fonts: Vec<Blob<u8>>,
    ) -> (Self, WindowId, Box<dyn Widget>) {
        let window_id = WindowId::next();
        let first_view = (logic)(&mut state);
        let mut ctx = ViewCtx {
            widget_map: WidgetMap::default(),
            id_path: Vec::new(),
            proxy: Arc::new(WindowProxy(window_id, MasonryProxy(Box::new(event_sink)))),
            runtime,
            state_changed: true,
        };
        let (pod, view_state) = first_view.build(&mut ctx);
        let driver = Self {
            logic,
            state,
            current_view: first_view,
            ctx,
            view_state,
            fonts,
            window_id,
        };
        (
            driver,
            window_id,
            Box::new(RootWidget::from_pod(pod.into_widget_pod().erased())),
        )
    }
}

/// The `WidgetId` which async events should be sent to.
pub const ASYNC_MARKER_WIDGET: WidgetId = WidgetId::reserved(0x1000);

/// The action which should be used for async events.
pub fn async_action(path: Arc<[ViewId]>, message: DynMessage) -> masonry_winit::core::Action {
    masonry_winit::core::Action::Other(Box::<MessagePackage>::new((path, message)))
}

/// The type used to send a message for async events.
type MessagePackage = (Arc<[ViewId]>, DynMessage);

impl MasonryProxy {
    fn send_message(
        &self,
        window_id: WindowId,
        path: Arc<[ViewId]>,
        message: DynMessage,
    ) -> Result<(), ProxyError> {
        match (self.0)(MasonryUserEvent::Action(
            window_id,
            async_action(path, message),
            ASYNC_MARKER_WIDGET,
        )) {
            Ok(()) => Ok(()),
            Err(err) => {
                let MasonryUserEvent::Action(_, masonry_winit::core::Action::Other(res), _) = err
                else {
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

struct MasonryProxy(
    pub(crate) Box<dyn Fn(MasonryUserEvent) -> Result<(), MasonryUserEvent> + Send + Sync>,
);

impl Debug for MasonryProxy {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        f.debug_tuple("MasonryProxy").finish_non_exhaustive()
    }
}

#[derive(Debug)]
struct WindowProxy(pub(crate) WindowId, pub(crate) MasonryProxy);

impl RawProxy for WindowProxy {
    fn send_message(
        &self,
        path: Arc<[ViewId]>,
        message: xilem_core::DynMessage,
    ) -> Result<(), xilem_core::ProxyError> {
        self.1.send_message(self.0, path, message)
    }

    fn dyn_debug(&self) -> &dyn Debug {
        self
    }
}

impl<State, Logic, View> AppDriver for MasonryDriver<State, Logic, View, View::ViewState>
where
    Logic: FnMut(&mut State) -> View,
    View: WidgetView<State>,
{
    fn on_action(
        &mut self,
        window_id: WindowId,
        masonry_ctx: &mut masonry_winit::app::DriverCtx<'_, '_>,
        widget_id: WidgetId,
        action: masonry_winit::core::Action,
    ) {
        debug_assert_eq!(window_id, self.window_id, "unknown window");

        let message_result = if widget_id == ASYNC_MARKER_WIDGET {
            let masonry_winit::core::Action::Other(action) = action else {
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
                DynMessage::new(action),
                &mut self.state,
            )
        } else {
            tracing::error!(
                "Got action {action:?} for unknown widget. Did you forget to use `with_action_widget`?"
            );
            return;
        };
        let stashed_view;
        let rebuild_from = match message_result {
            // The semantics here haven't exactly been worked out.
            // This version of the implementation is based on the assumptions that:
            // 1) `MessageResult::Action` means that the app's state has changed (and so the logic needs to be reran)
            // 2) `MessageResult::RequestRebuild` requires that the app state is *not* rebuilt; this allows
            //     avoiding infinite loops.
            MessageResult::Action(()) => {
                let next_view = (self.logic)(&mut self.state);
                self.ctx.state_changed = true;
                stashed_view = std::mem::replace(&mut self.current_view, next_view);

                Some(&stashed_view)
            }
            MessageResult::RequestRebuild => {
                self.ctx.state_changed = false;
                Some(&self.current_view)
            }
            MessageResult::Nop => None,
            MessageResult::Stale(_) => {
                tracing::info!("Discarding message");
                None
            }
        };
        if let Some(prior_view) = rebuild_from {
            masonry_ctx
                .render_root(self.window_id)
                .edit_root_widget(|mut root| {
                    let mut root = root.downcast::<RootWidget>();
                    self.current_view.rebuild(
                        prior_view,
                        &mut self.view_state,
                        &mut self.ctx,
                        RootWidget::child_mut(&mut root).downcast(),
                    );
                });
        }
        if cfg!(debug_assertions)
            && rebuild_from.is_some()
            && !masonry_ctx
                .render_root(self.window_id)
                .needs_rewrite_passes()
        {
            tracing::debug!("Nothing changed as result of action");
        }
    }
    fn on_start(&mut self, state: &mut MasonryState) {
        // self.fonts is never used again, so we may as well deallocate it.
        let fonts = std::mem::take(&mut self.fonts);

        for root in state.roots() {
            // Register all provided fonts
            for font in &fonts {
                // We currently don't do anything with the resulting family information,
                // because we don't have an easy way to return this to the application.
                drop(root.register_fonts(font.clone()));
            }
        }
    }
}
