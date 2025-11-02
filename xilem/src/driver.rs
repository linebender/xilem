// Copyright 2024 the Xilem Authors
// SPDX-License-Identifier: Apache-2.0

#![expect(missing_docs, reason = "TODO - Document these items")]

use std::collections::{HashMap, HashSet};
use std::fmt::Debug;
use std::sync::Arc;

use masonry::app::FocusFallbackPolicy;
use masonry::core::{ErasedAction, WidgetId};
use masonry::peniko::Blob;
use masonry_winit::app::{
    AppDriver, DriverCtx, MasonryState, MasonryUserEvent, NewWindow, WindowId,
};

use crate::core::{
    AnyViewState, DynMessage, MessageContext, MessageResult, ProxyError, RawProxy, SendMessage,
    View, ViewId, ViewPathTracker,
};
use crate::window_view::WindowView;
use crate::{AppState, ViewCtx};

pub struct MasonryDriver<State, Logic> {
    state: State,
    logic: Logic,
    windows: HashMap<WindowId, Window<State>>,
    proxy: Arc<MasonryProxy>,
    runtime: Arc<tokio::runtime::Runtime>,
    // Fonts which will be registered on startup.
    fonts: Vec<Blob<u8>>,
    scratch_id_path: Vec<ViewId>,
}

struct Window<State> {
    view: WindowView<State>,
    view_ctx: ViewCtx,
    view_state: AnyViewState,
}

impl<State: 'static, Logic, WindowIter> MasonryDriver<State, Logic>
where
    State: 'static,
    Logic: FnMut(&mut State) -> WindowIter,
    WindowIter: Iterator<Item = WindowView<State>>,
{
    pub(crate) fn new(
        state: State,
        logic: Logic,
        // TODO: narrow down MasonryUserEvent in event_sink once masonry_winit supports custom event types
        // (we only ever use it to send MasonryUserEvent::Action with ASYNC_MARKER_WIDGET)
        event_sink: impl Fn(MasonryUserEvent) -> Result<(), MasonryUserEvent> + Send + Sync + 'static,
        runtime: Arc<tokio::runtime::Runtime>,
        fonts: Vec<Blob<u8>>,
    ) -> (Self, Vec<NewWindow>) {
        let mut driver = Self {
            state,
            logic,
            windows: HashMap::new(),
            proxy: Arc::new(MasonryProxy(Box::new(event_sink))),
            runtime,
            fonts,
            scratch_id_path: Vec::new(),
        };
        let windows: Vec<_> = (driver.logic)(&mut driver.state)
            .map(|view| driver.build_window(view))
            .collect();
        (driver, windows)
    }
}

/// The `WidgetId` which async events should be sent to.
pub const ASYNC_MARKER_WIDGET: WidgetId = WidgetId::reserved(0x1000);

/// The action which should be used for async events.
pub fn async_action(path: Arc<[ViewId]>, message: SendMessage) -> ErasedAction {
    Box::<MessagePackage>::new((path, message))
}

/// The type used to send a message for async events.
type MessagePackage = (Arc<[ViewId]>, SendMessage);

impl MasonryProxy {
    fn send_message(
        &self,
        window_id: WindowId,
        path: Arc<[ViewId]>,
        message: SendMessage,
    ) -> Result<(), ProxyError> {
        match (self.0)(MasonryUserEvent::Action(
            window_id,
            async_action(path, message),
            ASYNC_MARKER_WIDGET,
        )) {
            Ok(()) => Ok(()),
            Err(err) => {
                let MasonryUserEvent::Action(_, res, _) = err else {
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
struct WindowProxy(WindowId, Arc<MasonryProxy>);

impl RawProxy for WindowProxy {
    fn send_message(&self, path: Arc<[ViewId]>, message: SendMessage) -> Result<(), ProxyError> {
        self.1.send_message(self.0, path, message)
    }

    fn dyn_debug(&self) -> &dyn Debug {
        self
    }
}

impl<State, Logic, WindowIter> MasonryDriver<State, Logic>
where
    State: 'static,
    Logic: FnMut(&mut State) -> WindowIter,
    WindowIter: Iterator<Item = WindowView<State>>,
{
    fn build_window(&mut self, window_view: WindowView<State>) -> NewWindow {
        let mut view_ctx = ViewCtx::new(
            Arc::new(WindowProxy(window_view.id, self.proxy.clone())),
            self.runtime.clone(),
        );
        let (new_window, view_state) = window_view.build(&mut view_ctx, &mut self.state);
        self.windows.insert(
            window_view.id,
            Window {
                view: window_view,
                view_ctx,
                view_state,
            },
        );
        new_window.0
    }

    pub(crate) fn create_window(
        &mut self,
        driver_ctx: &mut DriverCtx<'_, '_>,
        view: WindowView<State>,
    ) {
        driver_ctx.create_window(self.build_window(view));
    }

    fn close_window(&mut self, window_id: WindowId, ctx: &mut DriverCtx<'_, '_>) {
        let window = self.windows.get_mut(&window_id).unwrap();
        window.view.teardown(
            &mut window.view_state,
            &mut window.view_ctx,
            ctx.window(window_id),
        );
        self.windows.remove(&window_id);
        ctx.close_window(window_id);
    }

    fn run_logic(&mut self, driver_ctx: &mut DriverCtx<'_, '_>) {
        let mut returned_ids = HashSet::new();
        for next_view in (self.logic)(&mut self.state) {
            if !returned_ids.insert(next_view.id) {
                tracing::error!(
                    window_id = next_view.id.trace(),
                    "logic function returned two windows with the same id, ignoring the duplicate"
                );
                continue;
            }

            match self.windows.get_mut(&next_view.id) {
                Some(Window {
                    view,
                    view_ctx,
                    view_state,
                }) => {
                    next_view.rebuild(
                        view,
                        view_state,
                        view_ctx,
                        driver_ctx.window(next_view.id),
                        &mut self.state,
                    );
                    *view = next_view;
                }
                None => self.create_window(driver_ctx, next_view),
            }
        }

        let to_be_closed: Vec<_> = self
            .windows
            .keys()
            .copied()
            .filter(|id| !returned_ids.contains(id))
            .collect();
        for window_id in to_be_closed {
            self.close_window(window_id, driver_ctx);
        }
    }
}

impl<State, Logic, WindowIter> AppDriver for MasonryDriver<State, Logic>
where
    State: AppState + 'static,
    Logic: FnMut(&mut State) -> WindowIter,
    WindowIter: Iterator<Item = WindowView<State>>,
{
    fn on_action(
        &mut self,
        window_id: WindowId,
        masonry_ctx: &mut DriverCtx<'_, '_>,
        widget_id: WidgetId,
        action: ErasedAction,
    ) {
        let Some(window) = self.windows.get_mut(&window_id) else {
            tracing::warn!(
                window_id = window_id.trace(),
                "call on_action call for unknown window"
            );
            return;
        };

        let mut id_path = std::mem::take(&mut self.scratch_id_path);
        id_path.clear();
        let message_result = if widget_id == ASYNC_MARKER_WIDGET {
            // If this is not an action from a real widget, dispatch it using the path it contains.
            let (path, message) = *action.downcast::<MessagePackage>().unwrap();
            id_path.extend_from_slice(&path);
            let mut message_context = MessageContext::new(
                std::mem::take(window.view_ctx.environment()),
                id_path,
                message.into(),
            );
            let res = window.view.message(
                &mut window.view_state,
                &mut message_context,
                masonry_ctx.window(window_id),
                &mut self.state,
            );
            let (env, id_path, _message) = message_context.finish();
            *window.view_ctx.environment() = env;
            self.scratch_id_path = id_path;
            // TODO: Handle `message` somehow?
            res
        } else if let Some(path) = window.view_ctx.get_id_path(widget_id) {
            id_path.extend_from_slice(path);
            let mut message_context = MessageContext::new(
                std::mem::take(window.view_ctx.environment()),
                id_path,
                DynMessage(action),
            );
            let res = window.view.message(
                &mut window.view_state,
                &mut message_context,
                masonry_ctx.window(window_id),
                &mut self.state,
            );
            let (env, id_path, _message) = message_context.finish();
            *window.view_ctx.environment() = env;
            self.scratch_id_path = id_path;
            // TODO: Handle `message` somehow?
            res
        } else {
            tracing::error!(
                "Got action {action:?} for unknown widget. Did you forget to use `with_action_widget`?"
            );
            return;
        };
        match message_result {
            // The semantics here haven't exactly been worked out.
            // This version of the implementation is based on the assumptions that:
            // 1) `MessageResult::Action` means that the app's state has changed (and so the logic needs to be reran)
            // 2) `MessageResult::RequestRebuild` requires that the app state is *not* rebuilt; this allows
            //     avoiding infinite loops.
            MessageResult::Action(()) => {
                self.run_logic(masonry_ctx);
            }
            MessageResult::RequestRebuild => {
                window.view_ctx.set_state_changed(false);
                window.view.rebuild_root_widget(
                    &window.view,
                    &mut window.view_state,
                    &mut window.view_ctx,
                    masonry_ctx.render_root(window_id),
                    &mut self.state,
                );
            }
            MessageResult::Nop => {}
            MessageResult::Stale => {
                tracing::info!("Discarding message");
            }
        };
    }

    fn on_start(&mut self, state: &mut MasonryState<'_>) {
        // self.fonts is never used again, so we may as well deallocate it.
        let fonts = std::mem::take(&mut self.fonts);

        for root in state.roots() {
            // Register all provided fonts
            for font in &fonts {
                // We currently don't do anything with the resulting family information,
                // because we don't have an easy way to return this to the application.
                drop(root.register_fonts(font.clone()));
            }

            // Provide an initial sensible fallback for Xilem apps: first text input in the tree.
            let _ = root.set_focus_fallback_policy(FocusFallbackPolicy::FirstTextInput);
        }
    }

    fn on_close_requested(&mut self, window_id: WindowId, ctx: &mut DriverCtx<'_, '_>) {
        let view = &self.windows.get(&window_id).unwrap().view;
        view.on_close(&mut self.state);
        self.run_logic(ctx);

        if !self.state.keep_running() {
            // TODO: we should probably call teardown for all windows before exiting => introduce AppDriver::on_exit
            ctx.exit();
        }
    }
}
