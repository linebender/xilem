// Copyright 2024 the Xilem Authors
// SPDX-License-Identifier: Apache-2.0

#![expect(missing_docs, reason = "TODO - Document these items")]

use std::collections::{HashMap, HashSet};
use std::fmt::Debug;
use std::sync::Arc;

use masonry::core::{AnyWidget, WidgetId, WidgetPod};
use masonry::peniko::Blob;
use masonry_winit::app::{Action, AppDriver, DriverCtx, MasonryState, MasonryUserEvent, WindowId};
use winit::window::WindowAttributes;
use xilem_core::{AnyViewState, RawProxy, View};

use crate::core::{DynMessage, MessageResult, ProxyError, ViewId};
use crate::window_view::{CreateWindow, WindowView};
use crate::{AnyWidgetView, AppState, ViewCtx, WindowOptions};

pub struct MasonryDriver<State, Logic> {
    state: State,
    logic: Logic,
    windows: HashMap<WindowId, Window<State>>,
    proxy: Arc<MasonryProxy>,
    runtime: Arc<tokio::runtime::Runtime>,
    // Fonts which will be registered on startup.
    fonts: Vec<Blob<u8>>,
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
    WindowIter: Iterator<Item = (WindowId, WindowOptions<State>, Box<AnyWidgetView<State>>)>,
{
    pub(crate) fn new(
        state: State,
        logic: Logic,
        // TODO: narrow down MasonryUserEvent in event_sink once masonry_winit supports custom event types
        // (we only ever use it to send MasonryUserEvent::Action with ASYNC_MARKER_WIDGET)
        event_sink: impl Fn(MasonryUserEvent) -> Result<(), MasonryUserEvent> + Send + Sync + 'static,
        runtime: tokio::runtime::Runtime,
        fonts: Vec<Blob<u8>>,
    ) -> (
        Self,
        Vec<(WindowId, WindowAttributes, WidgetPod<dyn AnyWidget>)>,
    ) {
        let mut driver = Self {
            state,
            logic,
            windows: HashMap::new(),
            proxy: Arc::new(MasonryProxy(Box::new(event_sink))),
            runtime: Arc::new(runtime),
            fonts,
        };
        let windows: Vec<_> = (driver.logic)(&mut driver.state)
            .map(|(id, attrs, root_widget_view)| {
                let view = WindowView::new(attrs, root_widget_view);
                let (attrs, root_widget) = driver.build_window(id, view);
                (id, attrs, root_widget)
            })
            .collect();
        (driver, windows)
    }
}

/// The `WidgetId` which async events should be sent to.
pub const ASYNC_MARKER_WIDGET: WidgetId = WidgetId::reserved(0x1000);

/// The action which should be used for async events.
pub fn async_action(path: Arc<[ViewId]>, message: DynMessage) -> Action {
    Box::<MessagePackage>::new((path, message))
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

impl<State, Logic, WindowIter> MasonryDriver<State, Logic>
where
    State: 'static,
    Logic: FnMut(&mut State) -> WindowIter,
    WindowIter: Iterator<Item = (WindowId, WindowOptions<State>, Box<AnyWidgetView<State>>)>,
{
    fn build_window(
        &mut self,
        window_id: WindowId,
        view: WindowView<State>,
    ) -> (WindowAttributes, WidgetPod<dyn AnyWidget>) {
        let mut view_ctx = ViewCtx::new(
            Arc::new(WindowProxy(window_id, self.proxy.clone())),
            self.runtime.clone(),
        );
        let (CreateWindow(attrs, root_widget), view_state) =
            view.build(&mut view_ctx, &mut self.state);
        self.windows.insert(
            window_id,
            Window {
                view,
                view_ctx,
                view_state,
            },
        );
        (attrs, root_widget)
    }

    pub(crate) fn create_window(
        &mut self,
        driver_ctx: &mut DriverCtx<'_, '_>,
        window_id: WindowId,
        view: WindowView<State>,
    ) {
        let (attrs, root_widget) = self.build_window(window_id, view);
        driver_ctx.create_window(window_id, attrs, root_widget);
    }

    fn close_window(&mut self, window_id: WindowId, ctx: &mut DriverCtx<'_, '_>) {
        let window = self.windows.get_mut(&window_id).unwrap();
        window.view.teardown(
            &mut window.view_state,
            &mut window.view_ctx,
            ctx.window_handle_and_render_root(window_id),
            &mut self.state,
        );
        self.windows.remove(&window_id);
        ctx.close_window(window_id);
    }

    fn run_logic(&mut self, driver_ctx: &mut DriverCtx<'_, '_>) {
        let mut returned_ids = HashSet::new();
        for (window_id, next_attrs, next_view) in (self.logic)(&mut self.state) {
            if !returned_ids.insert(window_id) {
                tracing::error!(
                    window_id = window_id.trace(),
                    "logic function returned two windows with the same id, ignoring the duplicate"
                );
                continue;
            }
            let next_view = WindowView::new(next_attrs, next_view);

            match self.windows.get_mut(&window_id) {
                Some(Window {
                    view,
                    view_ctx,
                    view_state,
                }) => {
                    next_view.rebuild(
                        view,
                        view_state,
                        view_ctx,
                        driver_ctx.window_handle_and_render_root(window_id),
                        &mut self.state,
                    );
                    *view = next_view;
                }
                None => self.create_window(driver_ctx, window_id, next_view),
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
    WindowIter: Iterator<Item = (WindowId, WindowOptions<State>, Box<AnyWidgetView<State>>)>,
{
    fn on_action(
        &mut self,
        window_id: WindowId,
        masonry_ctx: &mut masonry_winit::app::DriverCtx<'_, '_>,
        widget_id: WidgetId,
        action: Action,
    ) {
        let Some(window) = self.windows.get_mut(&window_id) else {
            tracing::warn!(
                window_id = window_id.trace(),
                "call on_action call for unknown window"
            );
            return;
        };

        let message_result = if widget_id == ASYNC_MARKER_WIDGET {
            let (path, message) = *action.downcast::<MessagePackage>().unwrap();
            // Handle an async path
            window
                .view
                .message(&mut window.view_state, &path, message, &mut self.state)
        } else if let Some(id_path) = window.view_ctx.get_id_path(widget_id) {
            window.view.message(
                &mut window.view_state,
                id_path.as_slice(),
                DynMessage(action),
                &mut self.state,
            )
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
            MessageResult::Stale(_) => {
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
        }
    }

    fn on_close_requested(
        &mut self,
        window_id: WindowId,
        ctx: &mut masonry_winit::app::DriverCtx<'_, '_>,
    ) {
        let view = &self.windows.get(&window_id).unwrap().view;
        view.on_close(&mut self.state);
        self.run_logic(ctx);

        if !self.state.keep_running() {
            // TODO: we should probably call teardown for all windows before exiting => introduce AppDriver::on_exit
            ctx.exit();
        }
    }
}
