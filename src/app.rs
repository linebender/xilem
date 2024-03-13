// Copyright 2022 The Druid Authors.
//
// Licensed under the Apache License, Version 2.0 (the "License");
// you may not use this file except in compliance with the License.
// You may obtain a copy of the License at
//
//     http://www.apache.org/licenses/LICENSE-2.0
//
// Unless required by applicable law or agreed to in writing, software
// distributed under the License is distributed on an "AS IS" BASIS,
// WITHOUT WARRANTIES OR CONDITIONS OF ANY KIND, either express or implied.
// See the License for the specific language governing permissions and
// limitations under the License.

use std::collections::HashSet;
use std::time::Duration;

use parley::FontContext;
use tokio::runtime::Runtime;
use vello::{
    kurbo::{Point, Rect, Size},
    Scene,
};
use xilem_core::{AsyncWake, MessageResult};

use crate::widget::{
    BoxConstraints, CxState, EventCx, LayoutCx, LifeCycle, LifeCycleCx, PaintCx, Pod, PodFlags,
    UpdateCx, ViewContext, WidgetState,
};
use crate::{
    view::{Cx, Id, View},
    widget::Event,
};
use crate::{IdPath, Message};

/// App is the native backend implementation of Xilem. It contains the code interacting with glazier
/// and vello.
///
///
pub struct App<T, V: View<T>> {
    req_chan: tokio::sync::mpsc::Sender<AppReq>,
    response_chan: tokio::sync::mpsc::Receiver<RenderResponse<V, V::State>>,
    return_chan: tokio::sync::mpsc::Sender<(V, V::State, HashSet<Id>)>,
    id: Option<Id>,
    events: Vec<Message>,
    root_state: WidgetState,
    root_pod: Option<Pod>,
    size: Size,
    new_size: Size,
    cursor_pos: Option<Point>,
    cx: Cx,
    font_cx: FontContext,
    pub(crate) rt: Runtime,
}

/// The standard delay for waiting for async futures.
const RENDER_DELAY: Duration = Duration::from_millis(5);

/// This is the view logic of Xilem.
///
/// It contains no information about how to interact with the User (browser, native, terminal).
/// It is created by [`App`] and kept in a separate task for updating the apps contents.
/// The App can send [AppReq] to inform the the AppTask about an user interaction.
struct AppTask<T, V: View<T>, F: FnMut(&mut T) -> V> {
    req_chan: tokio::sync::mpsc::Receiver<AppReq>,
    response_chan: tokio::sync::mpsc::Sender<RenderResponse<V, V::State>>,
    return_chan: tokio::sync::mpsc::Receiver<(V, V::State, HashSet<Id>)>,

    data: T,
    app_logic: F,
    view: Option<V>,
    state: Option<V::State>,
    pending_async: HashSet<Id>,
    ui_state: UiState,
}

/// A message sent from the main UI thread ([`App`]) to the [`AppTask`].
pub(crate) enum AppReq {
    Events(Vec<Message>),
    Wake(IdPath),
    // Parameter indicates whether it should be delayed for async
    Render(bool),
}

/// A message sent from [`AppTask`] to [`App`] in response to a render request.
struct RenderResponse<V, S> {
    prev: Option<V>,
    view: V,
    state: Option<S>,
}

/// The state of the  [`AppTask`].
///
/// While the [`App`] follows a strict order of UIEvents -> Render -> Paint (this is simplified)
/// the [`AppTask`] can receive different requests at any time. This enum keeps track of the state
/// the AppTask is in because of previous requests.
#[derive(PartialEq)]
enum UiState {
    /// Starting state, ready for events and render requests.
    Start,
    /// Received render request, haven't responded yet.
    Delayed,
    /// An async completion woke the UI thread.
    WokeUI,
}

impl<T: Send + 'static, V: View<T> + 'static> App<T, V>
where
    V::State: 'static,
{
    /// Create a new app instance.
    pub fn new(data: T, app_logic: impl FnMut(&mut T) -> V + Send + 'static) -> Self {
        // Create a new tokio runtime. Doing it here is hacky, we should allow
        // the client to do it.
        let rt = Runtime::new().unwrap();

        // Note: there is danger of deadlock if exceeded; think this through.
        const CHANNEL_SIZE: usize = 1000;
        let (req_tx, req_rx) = tokio::sync::mpsc::channel(CHANNEL_SIZE);
        let (response_tx, response_rx) = tokio::sync::mpsc::channel(1);
        let (return_tx, return_rx) = tokio::sync::mpsc::channel(1);

        // We have a separate thread to forward wake requests (mostly generated
        // by the custom waker when we poll) to the async task. Maybe there's a
        // better way, but this is expedient.
        //
        // It's a sync_channel because sender needs to be sync to work in an async
        // context. Consider crossbeam and flume channels as alternatives.
        let req_tx_clone = req_tx.clone();
        let (wake_tx, wake_rx) = std::sync::mpsc::sync_channel(10);
        std::thread::spawn(move || {
            while let Ok(id_path) = wake_rx.recv() {
                let _ = req_tx_clone.blocking_send(AppReq::Wake(id_path));
            }
        });
        let cx = Cx::new(&wake_tx);

        // spawn app task
        rt.spawn(async move {
            let mut app_task = AppTask {
                req_chan: req_rx,
                response_chan: response_tx,
                return_chan: return_rx,
                data,
                app_logic,
                view: None,
                state: None,
                pending_async: HashSet::new(),
                ui_state: UiState::Start,
            };
            app_task.run().await;
        });
        App {
            req_chan: req_tx,
            response_chan: response_rx,
            return_chan: return_tx,
            id: None,
            root_pod: None,
            events: Vec::new(),
            root_state: WidgetState::new(crate::id::Id::next()),
            size: Default::default(),
            new_size: Default::default(),
            cursor_pos: None,
            cx,
            font_cx: FontContext::new(),
            rt,
        }
    }

    pub fn size(&mut self, size: Size) {
        self.new_size = size;
    }

    /// Run a paint cycle for the application.
    ///
    /// This is not just painting, but involves processing events, doing layout
    /// if needed, updating the accessibility tree, and then actually painting.
    pub fn paint(&mut self) {
        loop {
            self.send_events();
            // TODO: be more lazy re-rendering
            self.render();
            let root_pod = self.root_pod.as_mut().unwrap();
            let mut cx_state =
                CxState::new(&mut self.font_cx, &self.cx.tree_structure, &mut self.events);

            let mut lifecycle_cx = LifeCycleCx::new(&mut cx_state, &mut self.root_state);
            root_pod.lifecycle(&mut lifecycle_cx, &LifeCycle::TreeUpdate);

            if root_pod.state.flags.contains(PodFlags::REQUEST_UPDATE) {
                let mut update_cx = UpdateCx::new(&mut cx_state, &mut self.root_state);
                root_pod.update(&mut update_cx);
            }
            if root_pod.state.flags.contains(PodFlags::REQUEST_LAYOUT) || self.size != self.new_size
            {
                self.size = self.new_size;
                let mut layout_cx = LayoutCx::new(&mut cx_state, &mut self.root_state);
                let bc = BoxConstraints::tight(self.size);
                root_pod.layout(&mut layout_cx, &bc);
                root_pod.set_origin(&mut layout_cx, Point::ORIGIN);
            }
            if root_pod
                .state
                .flags
                .contains(PodFlags::VIEW_CONTEXT_CHANGED)
            {
                let view_context = ViewContext {
                    window_origin: Point::ORIGIN,
                    clip: Rect::from_origin_size(Point::ORIGIN, root_pod.state.size),
                    mouse_position: self.cursor_pos,
                };
                let mut lifecycle_cx = LifeCycleCx::new(&mut cx_state, &mut self.root_state);
                root_pod.lifecycle(
                    &mut lifecycle_cx,
                    &LifeCycle::ViewContextChanged(view_context),
                );
            }

            if cx_state.has_messages() {
                // Rerun app logic, primarily for LayoutObserver
                // We might want some debugging here if the number of iterations
                // becomes extreme.
                continue;
            }

            // Borrow again to avoid multiple borrows.
            // TODO: maybe make accessibility a method on CxState?
            let root_pod = self.root_pod.as_mut().unwrap();
            let mut cx_state =
                CxState::new(&mut self.font_cx, &self.cx.tree_structure, &mut self.events);
            let mut paint_cx = PaintCx::new(&mut cx_state, &mut self.root_state);
            root_pod.paint_impl(&mut paint_cx);
            break;
        }
    }

    pub fn window_event(&mut self, event: Event) {
        match &event {
            Event::MouseUp(me)
            | Event::MouseMove(me)
            | Event::MouseDown(me)
            | Event::MouseWheel(me) => {
                self.cursor_pos = Some(me.pos);
            }
            Event::MouseLeft() => {
                self.cursor_pos = None;
            }
        }

        self.ensure_root();
        let root_pod = self.root_pod.as_mut().unwrap();
        let mut cx_state =
            CxState::new(&mut self.font_cx, &self.cx.tree_structure, &mut self.events);
        let mut event_cx = EventCx::new(&mut cx_state, &mut self.root_state);
        root_pod.event(&mut event_cx, &event);
        self.send_events();
    }

    fn send_events(&mut self) {
        if !self.events.is_empty() {
            let events = std::mem::take(&mut self.events);
            let _ = self.req_chan.blocking_send(AppReq::Events(events));
        }
    }

    // Make sure the widget tree (root pod) is available
    fn ensure_root(&mut self) {
        if self.root_pod.is_none() {
            self.render();
        }
    }

    /// Run the app logic and update the widget tree.
    fn render(&mut self) {
        if self.render_inner(false) {
            self.render_inner(true);
        }
    }

    /// Run one pass of app logic.
    ///
    /// Return value is whether there are any pending async futures.
    fn render_inner(&mut self, delay: bool) -> bool {
        self.cx.pending_async.clear();
        let _ = self.req_chan.blocking_send(AppReq::Render(delay));
        if let Some(response) = self.response_chan.blocking_recv() {
            let state = if let Some(root_pod) = self.root_pod.as_mut() {
                let mut state = response.state.unwrap();
                let changes = self.cx.with_pod(root_pod, |root_el, cx| {
                    response.view.rebuild(
                        cx,
                        response.prev.as_ref().unwrap(),
                        self.id.as_mut().unwrap(),
                        &mut state,
                        root_el,
                    )
                });
                let _ = root_pod.mark(changes);
                assert!(self.cx.id_path_is_empty(), "id path imbalance on rebuild");
                assert!(
                    self.cx.element_id_path_is_empty(),
                    "element id path imbalance on rebuild"
                );
                state
            } else {
                let (id, state, root_pod) = self.cx.with_new_pod(|cx| response.view.build(cx));
                assert!(self.cx.id_path_is_empty(), "id path imbalance on build");
                assert!(
                    self.cx.element_id_path_is_empty(),
                    "element id path imbalance on rebuild"
                );
                self.root_pod = Some(root_pod);
                self.id = Some(id);
                state
            };
            let pending = std::mem::take(&mut self.cx.pending_async);
            let has_pending = !pending.is_empty();
            let _ = self
                .return_chan
                .blocking_send((response.view, state, pending));
            has_pending
        } else {
            false
        }
    }
}

impl<T, V: View<T>> App<T, V> {
    pub fn fragment(&self) -> &Scene {
        &self.root_pod.as_ref().unwrap().fragment
    }
}

impl<T, V: View<T>, F: FnMut(&mut T) -> V> AppTask<T, V, F> {
    async fn run(&mut self) {
        let mut deadline = None;
        loop {
            let rx = self.req_chan.recv();
            let req = match deadline {
                Some(deadline) => tokio::time::timeout_at(deadline, rx).await,
                None => Ok(rx.await),
            };
            match req {
                Ok(Some(req)) => match req {
                    AppReq::Events(events) => {
                        for event in events {
                            let id_path = &event.id_path[1..];
                            self.view.as_ref().unwrap().message(
                                id_path,
                                self.state.as_mut().unwrap(),
                                event.body,
                                &mut self.data,
                            );
                        }
                    }
                    AppReq::Wake(id_path) => {
                        let needs_rebuild;
                        {
                            let result = self.view.as_ref().unwrap().message(
                                &id_path[1..],
                                self.state.as_mut().unwrap(),
                                Box::new(AsyncWake),
                                &mut self.data,
                            );
                            needs_rebuild = matches!(result, MessageResult::RequestRebuild);
                        }

                        if needs_rebuild {
                            // request re-render from UI thread
                            if self.ui_state == UiState::Start {
                                self.ui_state = UiState::WokeUI;
                            }
                            let id = id_path.last().unwrap();
                            self.pending_async.remove(id);
                            if self.pending_async.is_empty() && self.ui_state == UiState::Delayed {
                                self.render().await;
                                deadline = None;
                            }
                        }
                    }
                    AppReq::Render(delay) => {
                        if !delay || self.pending_async.is_empty() {
                            self.render().await;
                            deadline = None;
                        } else {
                            deadline = Some(tokio::time::Instant::now() + RENDER_DELAY);
                            self.ui_state = UiState::Delayed;
                        }
                    }
                },
                Ok(None) => break,
                Err(_) => {
                    self.render().await;
                    deadline = None;
                }
            }
        }
    }

    async fn render(&mut self) {
        let view = (self.app_logic)(&mut self.data);
        let response = RenderResponse {
            prev: self.view.take(),
            view,
            state: self.state.take(),
        };
        if self.response_chan.send(response).await.is_err() {
            println!("error sending render response");
        }
        if let Some((view, state, pending)) = self.return_chan.recv().await {
            self.view = Some(view);
            self.state = Some(state);
            self.pending_async = pending;
        }
        self.ui_state = UiState::Start;
    }
}
