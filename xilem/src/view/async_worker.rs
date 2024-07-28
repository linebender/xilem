// Copyright 2024 the Xilem Authors
// SPDX-License-Identifier: Apache-2.0

use std::{future::Future, marker::PhantomData, sync::Mutex};

use tokio::task::JoinHandle;
use xilem_core::{DynMessage, Message, MessageProxy, NoElement, View, ViewPathTracker};

use crate::ViewCtx;

/// Launches a task which will run until the view is no longer in the tree,
/// and allows to update app state in reaction to the events from that task.
///
/// `task` is given a [`MessageProxy`], and returns a future which will be
/// spawned when the view is built.
/// This `MessageProxy` can be used to send a message to `on_event`, which
/// can then update the app's state.
///
/// Note that this task will not be updated if the view is rebuilt.
pub fn async_worker<M, F, H, State, Action, Fut>(task: F, on_event: H) -> AsyncWorker<F, H, M>
where
    F: FnOnce(MessageProxy<M>) -> Fut,
    Fut: Future<Output = ()> + Send + 'static,
    H: Fn(&mut State, M) -> Action + 'static,
    M: Message + 'static,
{
    AsyncWorker {
        task: Mutex::new(Some(task)),
        on_event,
        message: PhantomData,
    }
}

pub struct AsyncWorker<F, H, M> {
    task: Mutex<Option<F>>,
    on_event: H,
    message: PhantomData<fn() -> M>,
}

impl<State, Action, F, H, M, Fut> View<State, Action, ViewCtx> for AsyncWorker<F, H, M>
where
    F: FnOnce(MessageProxy<M>) -> Fut + 'static,
    Fut: Future<Output = ()> + Send + 'static,
    H: Fn(&mut State, M) -> Action + 'static,
    M: Message + 'static,
{
    type Element = NoElement;

    type ViewState = Option<JoinHandle<()>>;

    fn build(&self, ctx: &mut ViewCtx) -> (Self::Element, Self::ViewState) {
        let handle = self.task.lock().unwrap().take().map(|task| {
            let proxy = MessageProxy::new(ctx.proxy.clone(), ctx.view_path().into());
            ctx.runtime().spawn((task)(proxy))
        });
        (NoElement, handle)
    }

    fn rebuild<'el>(
        &self,
        _: &Self,
        _: &mut Self::ViewState,
        _: &mut ViewCtx,
        (): xilem_core::Mut<'el, Self::Element>,
    ) -> xilem_core::Mut<'el, Self::Element> {
        // Nothing to do
    }

    fn teardown(
        &self,
        join_handle: &mut Self::ViewState,
        _: &mut ViewCtx,
        _: xilem_core::Mut<'_, Self::Element>,
    ) {
        join_handle.as_ref().map(JoinHandle::abort);
    }

    fn message(
        &self,
        _: &mut Self::ViewState,
        id_path: &[xilem_core::ViewId],
        message: DynMessage,
        app_state: &mut State,
    ) -> xilem_core::MessageResult<Action> {
        debug_assert!(
            id_path.is_empty(),
            "id path should be empty in AsyncRepeat::message"
        );
        let message = message.downcast::<M>().unwrap();
        xilem_core::MessageResult::Action((self.on_event)(app_state, *message))
    }
}
