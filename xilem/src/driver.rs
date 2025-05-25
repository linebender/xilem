// Copyright 2024 the Xilem Authors
// SPDX-License-Identifier: Apache-2.0

#![expect(missing_docs, reason = "TODO - Document these items")]

use std::collections::{HashMap, HashSet};
use std::fmt::Debug;
use std::sync::Arc;

use masonry_winit::app::{
    AppDriver, DriverCtx, EventLoopProxy, MasonryState, MasonryUserEvent, WindowId,
};
use masonry_winit::core::WidgetId;
use masonry_winit::peniko::Blob;
use xilem_core::{AnyViewState, RawProxy, View};

use crate::core::{DynMessage, MessageResult, ProxyError, ViewId};
use crate::window_view::{CreateWindow, WindowView};
use crate::{AnyWidgetView, ViewCtx, WidgetMap, WindowAttrs};

pub struct MasonryDriver<State, Logic> {
    pub(crate) state: State,
    pub(crate) logic: Logic,
    windows: HashMap<WindowId, Window<State>>,
    // Fonts which will be registered on startup.
    pub(crate) fonts: Vec<Blob<u8>>,
    proxy: Arc<MasonryProxy>,
    runtime: Arc<tokio::runtime::Runtime>,
    keep_running: Box<dyn Fn(&State) -> bool>,
}

struct Window<State> {
    view: WindowView<State>,
    view_ctx: ViewCtx,
    view_state: AnyViewState,
}

impl<State: 'static, Logic> MasonryDriver<State, Logic> {
    pub(crate) fn new(
        state: State,
        logic: Logic,
        fonts: Vec<Blob<u8>>,
        proxy: Arc<MasonryProxy>,
        runtime: Arc<tokio::runtime::Runtime>,
        keep_running: Box<dyn Fn(&State) -> bool>,
    ) -> Self {
        Self {
            state,
            logic,
            windows: HashMap::new(),
            fonts,
            proxy,
            runtime,
            keep_running,
        }
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
    pub(crate) fn send_message(
        &self,
        window_id: WindowId,
        path: Arc<[ViewId]>,
        message: DynMessage,
    ) -> Result<(), ProxyError> {
        match self.0.send_event(MasonryUserEvent::Action(
            window_id,
            async_action(path, message),
            ASYNC_MARKER_WIDGET,
        )) {
            Ok(()) => Ok(()),
            Err(err) => {
                let MasonryUserEvent::Action(_, masonry_winit::core::Action::Other(res), _) = err.0
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

#[derive(Debug)]
pub struct MasonryProxy(pub(crate) EventLoopProxy);

impl MasonryProxy {
    pub fn new(proxy: EventLoopProxy) -> Self {
        Self(proxy)
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
    WindowIter: Iterator<Item = (WindowId, WindowAttrs<State>, Box<AnyWidgetView<State>>)>,
{
    pub(crate) fn create_window(
        &mut self,
        driver_ctx: &mut DriverCtx<'_, '_>,
        window_id: WindowId,
        view: WindowView<State>,
    ) {
        let mut view_ctx = ViewCtx {
            widget_map: WidgetMap::default(),
            id_path: Vec::new(),
            proxy: Arc::new(WindowProxy(window_id, self.proxy.clone())),
            runtime: self.runtime.clone(),
            state_changed: true,
        };
        let (CreateWindow(attrs, root_widget), view_state) = view.build(&mut view_ctx);
        driver_ctx.create_window(window_id, root_widget, attrs);
        self.windows.insert(
            window_id,
            Window {
                view,
                view_ctx,
                view_state,
            },
        );
    }

    fn close_window(&mut self, window_id: WindowId, ctx: &mut DriverCtx<'_, '_>) {
        let window = self.windows.get_mut(&window_id).unwrap();
        window.view.teardown(
            &mut window.view_state,
            &mut window.view_ctx,
            ctx.window_handle_and_render_root(window_id),
        );
        self.windows.remove(&window_id);
        ctx.close_window(window_id);
    }

    fn run_logic(&mut self, driver_ctx: &mut DriverCtx<'_, '_>) {
        let mut returned_ids = HashSet::new();
        for (window_id, next_attrs, next_view) in (self.logic)(&mut self.state) {
            if !returned_ids.insert(window_id) {
                tracing::error!(
                    id = ?window_id,
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
    State: 'static,
    Logic: FnMut(&mut State) -> WindowIter,
    WindowIter: Iterator<Item = (WindowId, WindowAttrs<State>, Box<AnyWidgetView<State>>)>,
{
    fn create_initial_windows(&mut self, ctx: &mut DriverCtx<'_, '_>) {
        for (window_id, attrs, root_widget_view) in (self.logic)(&mut self.state) {
            self.create_window(ctx, window_id, WindowView::new(attrs, root_widget_view));
        }
    }

    fn on_action(
        &mut self,
        window_id: WindowId,
        masonry_ctx: &mut masonry_winit::app::DriverCtx<'_, '_>,
        widget_id: WidgetId,
        action: masonry_winit::core::Action,
    ) {
        let Some(window) = self.windows.get_mut(&window_id) else {
            tracing::warn!(
                id = ?window_id,
                "call on_action call for unknown window"
            );
            return;
        };

        let message_result = if widget_id == ASYNC_MARKER_WIDGET {
            let masonry_winit::core::Action::Other(action) = action else {
                panic!();
            };
            let (path, message) = *action.downcast::<MessagePackage>().unwrap();
            // Handle an async path
            window
                .view
                .message(&mut window.view_state, &path, message, &mut self.state)
        } else if let Some(id_path) = window.view_ctx.widget_map.get(&widget_id) {
            window.view.message(
                &mut window.view_state,
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
                window.view_ctx.state_changed = false;
                window.view.rebuild_root_widget(
                    &window.view,
                    &mut window.view_state,
                    &mut window.view_ctx,
                    masonry_ctx.render_root(window_id),
                );
            }
            MessageResult::Nop => {}
            MessageResult::Stale(_) => {
                tracing::info!("Discarding message");
            }
        };
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

    fn on_close_requested(
        &mut self,
        window_id: WindowId,
        ctx: &mut masonry_winit::app::DriverCtx<'_, '_>,
    ) {
        let view = &self.windows.get(&window_id).unwrap().view;

        if let Some(on_close) = &view.attributes.callbacks.on_close {
            on_close(&mut self.state);
            self.run_logic(ctx);
        }

        if !(self.keep_running)(&self.state) {
            // TODO: should we call teardown for all windows before exiting?
            ctx.exit();
        }
    }
}
