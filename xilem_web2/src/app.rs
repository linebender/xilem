// Copyright 2023 the Xilem Authors
// SPDX-License-Identifier: Apache-2.0

use std::{cell::RefCell, rc::Rc};

use crate::{AnyNode, DomView, PodMut};
use xilem_core::{DynMessage, MessageResult, ViewId, ViewPathTracker};

type IdPath = Vec<ViewId>;

pub struct Message {
    pub id_path: IdPath,
    pub body: DynMessage,
}

pub struct MessageThunk {
    id_path: IdPath,
    app_ref: Box<dyn AppRunner>,
}

impl MessageThunk {
    pub fn push_message(&self, message_body: impl xilem_core::Message) {
        let message = Message {
            id_path: self.id_path.clone(),
            body: Box::new(message_body),
        };
        self.app_ref.handle_message(message);
    }
}

#[derive(Default)]
pub struct ViewCtx {
    id_path: IdPath,
    app_ref: Option<Box<dyn AppRunner>>,
}

impl ViewCtx {
    pub fn message_thunk(&self) -> MessageThunk {
        MessageThunk {
            id_path: self.id_path.clone(),
            app_ref: self.app_ref.as_ref().unwrap().clone_box(),
        }
    }
    pub(crate) fn set_runner(&mut self, runner: impl AppRunner + 'static) {
        self.app_ref = Some(Box::new(runner));
    }
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
}

/// The type responsible for running your app.
pub struct App<T, V: DomView<T>, F: FnMut(&mut T) -> V>(Rc<RefCell<AppInner<T, V, F>>>);

struct AppInner<T, V: DomView<T>, F: FnMut(&mut T) -> V> {
    data: T,
    app_logic: F,
    view: Option<V>,
    state: Option<V::ViewState>,
    element: Option<V::Element>,
    cx: ViewCtx,
}

pub(crate) trait AppRunner {
    fn handle_message(&self, message: Message);

    fn clone_box(&self) -> Box<dyn AppRunner>;
}

impl<T: 'static, V: DomView<T> + 'static, F: FnMut(&mut T) -> V + 'static> Clone for App<T, V, F> {
    fn clone(&self) -> Self {
        App(self.0.clone())
    }
}

impl<T: 'static, V: DomView<T> + 'static, F: FnMut(&mut T) -> V + 'static> App<T, V, F> {
    /// Create an instance of your app with the given logic and initial state.
    pub fn new(data: T, app_logic: F) -> Self {
        let inner = AppInner::new(data, app_logic);
        let app = App(Rc::new(RefCell::new(inner)));
        app.0.borrow_mut().cx.set_runner(app.clone());
        app
    }

    /// Run the app.
    ///
    /// Because we don't want to block the render thread, we return immediately here. The app is
    /// forgotten, and will continue to respond to events in the background.
    pub fn run(self, root: &web_sys::HtmlElement) {
        self.0.borrow_mut().ensure_app(root);
        // Latter may not be necessary, we have an rc loop.
        std::mem::forget(self);
    }
}

impl<T, V: DomView<T>, F: FnMut(&mut T) -> V> AppInner<T, V, F> {
    pub fn new(data: T, app_logic: F) -> Self {
        let cx = ViewCtx::default();
        AppInner {
            data,
            app_logic,
            view: None,
            state: None,
            element: None,
            cx,
        }
    }

    fn ensure_app(&mut self, root: &web_sys::HtmlElement) {
        if self.view.is_none() {
            let view = (self.app_logic)(&mut self.data);
            let (element, state) = view.build(&mut self.cx);
            self.view = Some(view);
            self.state = Some(state);

            // TODO should the element provide a separate method to access reference instead?
            let node: &web_sys::Node = element.node.as_node_ref();
            root.append_child(node).unwrap();
            self.element = Some(element);
        }
    }
}

impl<T: 'static, V: DomView<T> + 'static, F: FnMut(&mut T) -> V + 'static> AppRunner
    for App<T, V, F>
{
    // For now we handle the message synchronously, but it would also
    // make sense to to batch them (for example with requestAnimFrame).
    fn handle_message(&self, message: Message) {
        let mut inner_guard = self.0.borrow_mut();
        let inner = &mut *inner_guard;
        if let Some(view) = &mut inner.view {
            let message_result = view.message(
                inner.state.as_mut().unwrap(),
                &message.id_path,
                message.body,
                &mut inner.data,
            );

            match message_result {
                MessageResult::Nop | MessageResult::Action(_) => {
                    // Nothing to do.
                }
                MessageResult::RequestRebuild => {
                    // TODO force a rebuild?
                }
                MessageResult::Stale(_) => {
                    // TODO perhaps inform the user that a stale request bubbled to the top?
                }
            }

            let new_view = (inner.app_logic)(&mut inner.data);
            let el = inner.element.as_mut().unwrap();
            let pod_mut = PodMut::new(&mut el.node, &mut el.props, false);
            new_view.rebuild(view, inner.state.as_mut().unwrap(), &mut inner.cx, pod_mut);
            *view = new_view;
        }
    }

    fn clone_box(&self) -> Box<dyn AppRunner> {
        Box::new(self.clone())
    }
}
