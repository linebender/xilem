// Copyright 2023 the Xilem Authors
// SPDX-License-Identifier: Apache-2.0

use crate::{
    core::{AppendVec, MessageResult, ViewId},
    elements::DomChildrenSplice,
    AnyPod, DomFragment, DynMessage, ViewCtx,
};
use std::{cell::RefCell, rc::Rc};
use wasm_bindgen::UnwrapThrowExt;

pub(crate) struct AppMessage {
    pub id_path: Rc<[ViewId]>,
    pub body: DynMessage,
}

/// The type responsible for running your app.
pub struct App<State, Fragment: DomFragment<State>, InitFragment>(
    Rc<RefCell<AppInner<State, Fragment, InitFragment>>>,
);

struct AppInner<State, Fragment: DomFragment<State>, InitFragment> {
    data: State,
    root: web_sys::Node,
    app_logic: InitFragment,
    fragment: Option<Fragment>,
    fragment_state: Option<Fragment::SeqState>,
    fragment_append_scratch: AppendVec<AnyPod>,
    vec_splice_scratch: Vec<AnyPod>,
    elements: Vec<AnyPod>,
    ctx: ViewCtx,
}

pub(crate) trait AppRunner {
    fn handle_message(&self, message: AppMessage);

    fn clone_box(&self) -> Box<dyn AppRunner>;
}

impl<State, Fragment: DomFragment<State>, InitFragment> Clone
    for App<State, Fragment, InitFragment>
{
    fn clone(&self) -> Self {
        App(self.0.clone())
    }
}

impl<State, Fragment, InitFragment> App<State, Fragment, InitFragment>
where
    State: 'static,
    Fragment: DomFragment<State> + 'static,
    InitFragment: FnMut(&mut State) -> Fragment + 'static,
{
    /// Create an instance of your app with the given logic and initial state.
    pub fn new(root: impl AsRef<web_sys::Node>, data: State, app_logic: InitFragment) -> Self {
        let inner = AppInner::new(root.as_ref().clone(), data, app_logic);
        let app = App(Rc::new(RefCell::new(inner)));
        app.0.borrow_mut().ctx.set_runner(app.clone());
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

impl<State, Fragment: DomFragment<State>, InitFragment: FnMut(&mut State) -> Fragment>
    AppInner<State, Fragment, InitFragment>
{
    pub fn new(root: web_sys::Node, data: State, app_logic: InitFragment) -> Self {
        let ctx = ViewCtx::default();
        AppInner {
            data,
            root,
            app_logic,
            fragment: None,
            fragment_state: None,
            elements: Vec::new(),
            ctx,
            fragment_append_scratch: Default::default(),
            vec_splice_scratch: Default::default(),
        }
    }

    fn ensure_app(&mut self) {
        if self.fragment.is_none() {
            let fragment = (self.app_logic)(&mut self.data);
            let state = fragment.seq_build(&mut self.ctx, &mut self.fragment_append_scratch);
            self.fragment = Some(fragment);
            self.fragment_state = Some(state);

            // TODO should the element provide a separate method to access reference instead?
            let append_vec = std::mem::take(&mut self.fragment_append_scratch);

            self.elements = append_vec.into_inner();
            for pod in &self.elements {
                self.root.append_child(pod.node.as_ref()).unwrap_throw();
            }
        }
    }
}

impl<State, Fragment, InitFragment> AppRunner for App<State, Fragment, InitFragment>
where
    State: 'static,
    Fragment: DomFragment<State> + 'static,
    InitFragment: FnMut(&mut State) -> Fragment + 'static,
{
    // For now we handle the message synchronously, but it would also
    // make sense to to batch them (for example with requestAnimFrame).
    fn handle_message(&self, message: AppMessage) {
        let mut inner_guard = self.0.borrow_mut();
        let inner = &mut *inner_guard;
        if let Some(fragment) = &mut inner.fragment {
            let message_result = fragment.seq_message(
                inner.fragment_state.as_mut().unwrap(),
                &message.id_path,
                message.body,
                &mut inner.data,
            );

            // Each of those results are currently resulting in a rebuild, that may be subject to change
            match message_result {
                MessageResult::RequestRebuild | MessageResult::Nop | MessageResult::Action(_) => {}
                MessageResult::Stale(_) => {
                    // TODO perhaps inform the user that a stale request bubbled to the top?
                }
            }

            let new_fragment = (inner.app_logic)(&mut inner.data);
            let mut dom_children_splice = DomChildrenSplice::new(
                &mut inner.fragment_append_scratch,
                &mut inner.elements,
                &mut inner.vec_splice_scratch,
                &inner.root,
                inner.ctx.fragment.clone(),
                false,
                #[cfg(feature = "hydration")]
                false,
            );
            new_fragment.seq_rebuild(
                fragment,
                inner.fragment_state.as_mut().unwrap(),
                &mut inner.ctx,
                &mut dom_children_splice,
            );
            *fragment = new_fragment;
        }
    }

    fn clone_box(&self) -> Box<dyn AppRunner> {
        Box::new(self.clone())
    }
}
