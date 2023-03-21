// Copyright 2023 the Druid Authors.
// SPDX-License-Identifier: Apache-2.0

use std::{cell::RefCell, rc::Rc};

use crate::{
    context::Cx,
    view::{DomNode, View},
    Message,
};
use xilem_core::{Id, MessageResult};

pub struct App<T, V: View<T>, F: FnMut(&mut T) -> V>(Rc<RefCell<AppInner<T, V, F>>>);

struct AppInner<T, V: View<T>, F: FnMut(&mut T) -> V> {
    data: T,
    app_logic: F,
    view: Option<V>,
    id: Option<Id>,
    state: Option<V::State>,
    element: Option<V::Element>,
    cx: Cx,
}

pub(crate) trait AppRunner {
    fn handle_message(&self, message: Message);

    fn clone_box(&self) -> Box<dyn AppRunner>;
}

impl<T: 'static, V: View<T> + 'static, F: FnMut(&mut T) -> V + 'static> Clone for App<T, V, F> {
    fn clone(&self) -> Self {
        App(self.0.clone())
    }
}

impl<T: 'static, V: View<T> + 'static, F: FnMut(&mut T) -> V + 'static> App<T, V, F> {
    pub fn new(data: T, app_logic: F) -> Self {
        let inner = AppInner::new(data, app_logic);
        let app = App(Rc::new(RefCell::new(inner)));
        app.0.borrow_mut().cx.set_runner(app.clone());
        app
    }

    pub fn run(self, root: &web_sys::HtmlElement) {
        self.0.borrow_mut().ensure_app(root);
        // Latter may not be necessary, we have an rc loop.
        std::mem::forget(self)
    }
}

impl<T, V: View<T>, F: FnMut(&mut T) -> V> AppInner<T, V, F> {
    pub fn new(data: T, app_logic: F) -> Self {
        let cx = Cx::new();
        AppInner {
            data,
            app_logic,
            view: None,
            id: None,
            state: None,
            element: None,
            cx,
        }
    }

    fn ensure_app(&mut self, root: &web_sys::HtmlElement) {
        if self.view.is_none() {
            let view = (self.app_logic)(&mut self.data);
            let (id, state, element) = view.build(&mut self.cx);
            self.view = Some(view);
            self.id = Some(id);
            self.state = Some(state);

            root.append_child(element.as_node_ref()).unwrap();
            self.element = Some(element);
        }
    }
}

impl<T: 'static, V: View<T> + 'static, F: FnMut(&mut T) -> V + 'static> AppRunner for App<T, V, F> {
    // For now we handle the message synchronously, but it would also
    // make sense to to batch them (for example with requestAnimFrame).
    fn handle_message(&self, message: Message) {
        let mut inner_guard = self.0.borrow_mut();
        let inner = &mut *inner_guard;
        if let Some(view) = &mut inner.view {
            let message_result = view.message(
                &message.id_path[1..],
                inner.state.as_mut().unwrap(),
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
            let _changed = new_view.rebuild(
                &mut inner.cx,
                view,
                inner.id.as_mut().unwrap(),
                inner.state.as_mut().unwrap(),
                inner.element.as_mut().unwrap(),
            );
            // Not sure we have to do anything on changed, the rebuild
            // traversal should cause the DOM to update.
            *view = new_view;
        }
    }

    fn clone_box(&self) -> Box<dyn AppRunner> {
        Box::new(self.clone())
    }
}
