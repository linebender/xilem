// Copyright 2023 the Xilem Authors
// SPDX-License-Identifier: Apache-2.0

use crate::{DomNode, ViewCtx};
use std::{cell::RefCell, rc::Rc};

use crate::{DomView, DynMessage, PodMut};
use xilem_core::{MessageResult, ViewId};

pub(crate) struct AppMessage {
    pub id_path: Rc<[ViewId]>,
    pub body: DynMessage,
}

/// The type responsible for running your app.
pub struct App<T, V: DomView<T>, F: FnMut(&mut T) -> V>(Rc<RefCell<AppInner<T, V, F>>>);

struct AppInner<T, V: DomView<T>, F: FnMut(&mut T) -> V> {
    data: T,
    root: web_sys::Node,
    app_logic: F,
    view: Option<V>,
    state: Option<V::ViewState>,
    element: Option<V::Element>,
    cx: ViewCtx,
}

pub(crate) trait AppRunner {
    fn handle_message(&self, message: AppMessage);

    fn clone_box(&self) -> Box<dyn AppRunner>;
}

impl<T: 'static, V: DomView<T> + 'static, F: FnMut(&mut T) -> V + 'static> Clone for App<T, V, F> {
    fn clone(&self) -> Self {
        App(self.0.clone())
    }
}

impl<T: 'static, V: DomView<T> + 'static, F: FnMut(&mut T) -> V + 'static> App<T, V, F> {
    /// Create an instance of your app with the given logic and initial state.
    pub fn new(root: impl AsRef<web_sys::Node>, data: T, app_logic: F) -> Self {
        let inner = AppInner::new(root.as_ref().clone(), data, app_logic);
        let app = App(Rc::new(RefCell::new(inner)));
        app.0.borrow_mut().cx.set_runner(app.clone());
        app
    }

    /// Run the app.
    ///
    /// Because we don't want to block the render thread, we return immediately here. The app is
    /// forgotten, and will continue to respond to events in the background.
    pub fn run(self) {
        self.0.borrow_mut().ensure_app();
        // Latter may not be necessary, we have an rc loop.
        std::mem::forget(self);
    }
}

impl<T, V: DomView<T>, F: FnMut(&mut T) -> V> AppInner<T, V, F> {
    pub fn new(root: web_sys::Node, data: T, app_logic: F) -> Self {
        let cx = ViewCtx::default();
        AppInner {
            data,
            root,
            app_logic,
            view: None,
            state: None,
            element: None,
            cx,
        }
    }

    fn ensure_app(&mut self) {
        if self.view.is_none() {
            let view = (self.app_logic)(&mut self.data);
            let (mut element, state) = view.build(&mut self.cx);
            element.node.apply_props(&mut element.props);
            self.view = Some(view);
            self.state = Some(state);

            // TODO should the element provide a separate method to access reference instead?
            let node: &web_sys::Node = element.node.as_ref();
            self.root.append_child(node).unwrap();
            self.element = Some(element);
        }
    }
}

impl<T: 'static, V: DomView<T> + 'static, F: FnMut(&mut T) -> V + 'static> AppRunner
    for App<T, V, F>
{
    // For now we handle the message synchronously, but it would also
    // make sense to to batch them (for example with requestAnimFrame).
    fn handle_message(&self, message: AppMessage) {
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
            let pod_mut = PodMut::new(&mut el.node, &mut el.props, &inner.root, false);
            new_view.rebuild(view, inner.state.as_mut().unwrap(), &mut inner.cx, pod_mut);
            *view = new_view;
        }
    }

    fn clone_box(&self) -> Box<dyn AppRunner> {
        Box::new(self.clone())
    }
}
