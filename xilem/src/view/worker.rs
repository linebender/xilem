// Copyright 2024 the Xilem Authors
// SPDX-License-Identifier: Apache-2.0

use crate::{
    core::{
        DynMessage, Message, MessageProxy, MessageResult, Mut, NoElement, View, ViewId, ViewMarker,
        ViewPathTracker,
    },
    ViewCtx,
};
use std::{future::Future, marker::PhantomData, sync::Arc};
use tokio::{
    sync::mpsc::{UnboundedReceiver, UnboundedSender},
    task::JoinHandle,
};

/// Launch a task which will run until the view is no longer in the tree.
///
/// `init_future` is given a [`MessageProxy`], which it will store in the future it returns.
/// This `MessageProxy` can be used to send a message to `on_event`, which can then update
/// the app's state.
///
/// For example, this can be used with the time functions in [`crate::tokio::time`].
///
/// Note that this task will not be updated if the view is rebuilt, so `init_future`
/// cannot capture.
// TODO: More thorough documentation.
/// See [`run_once`](crate::core::run_once) for details.
pub fn worker<M, V, F, H, State, Action, Fut>(
    value: V,
    init_future: F,
    on_response: H,
) -> Worker<F, H, M, V>
where
    F: Fn(MessageProxy<M>, UnboundedReceiver<V>) -> Fut,
    Fut: Future<Output = ()> + Send + 'static,
    H: Fn(&mut State, M) -> Action + 'static,
    M: Message + 'static,
{
    const {
        assert!(
            core::mem::size_of::<F>() == 0,
            "`worker` will not be ran again when its captured variables are updated.\n\
            To ignore this warning, use `worker_raw`.
            To provide an updating value to this task, use the "
        );
    };
    Worker {
        value,
        init_future,
        on_response,
        message: PhantomData,
    }
}

/// Launch a worker which will run until the view is no longer in the tree.
///
/// This is [`worker`] without the capturing rules.
/// See `worker` for full documentation.
pub fn worker_raw<M, V, F, H, State, Action, Fut>(
    value: V,
    init_future: F,
    on_response: H,
) -> Worker<F, H, M, V>
where
    F: Fn(MessageProxy<M>, UnboundedReceiver<V>) -> Fut,
    Fut: Future<Output = ()> + Send + 'static,
    H: Fn(&mut State, M) -> Action + 'static,
    M: Message + 'static,
{
    Worker {
        value,
        init_future,
        on_response,
        message: PhantomData,
    }
}

pub struct Worker<F, H, M, V> {
    init_future: F,
    value: V,
    on_response: H,
    message: PhantomData<fn() -> M>,
}

#[doc(hidden)] // Implementation detail, public because of trait visibility rules
pub struct WorkerState<V> {
    handle: JoinHandle<()>,
    sender: UnboundedSender<V>,
}

impl<F, H, M, V> ViewMarker for Worker<F, H, M, V> {}

impl<State, Action, V, F, H, M, Fut> View<State, Action, ViewCtx> for Worker<F, H, M, V>
where
    F: Fn(MessageProxy<M>, UnboundedReceiver<V>) -> Fut + 'static,
    V: Send + PartialEq + Clone + 'static,
    Fut: Future<Output = ()> + Send + 'static,
    H: Fn(&mut State, M) -> Action + 'static,
    M: Message + 'static,
{
    type Element = NoElement;

    type ViewState = WorkerState<V>;

    fn build(&self, ctx: &mut ViewCtx) -> (Self::Element, Self::ViewState) {
        let path: Arc<[ViewId]> = ctx.view_path().into();

        let proxy = ctx.proxy.clone();
        let (tx, rx) = tokio::sync::mpsc::unbounded_channel();
        // No opportunity for the channel to be closed.
        tx.send(self.value.clone()).unwrap();
        let handle = ctx
            .runtime()
            .spawn((self.init_future)(MessageProxy::new(proxy, path), rx));
        (NoElement, WorkerState { handle, sender: tx })
    }

    fn rebuild(
        &self,
        prev: &Self,
        view_state: &mut Self::ViewState,
        _: &mut ViewCtx,
        (): Mut<Self::Element>,
    ) {
        if self.value != prev.value {
            // TODO: Error handling
            drop(view_state.sender.send(self.value.clone()));
        }
    }

    fn teardown(&self, view_state: &mut Self::ViewState, _: &mut ViewCtx, _: Mut<Self::Element>) {
        view_state.handle.abort();
    }

    fn message(
        &self,
        _: &mut Self::ViewState,
        id_path: &[ViewId],
        message: DynMessage,
        app_state: &mut State,
    ) -> MessageResult<Action> {
        debug_assert!(
            id_path.is_empty(),
            "id path should be empty in Task::message"
        );
        let message = message.downcast::<M>().unwrap();
        MessageResult::Action((self.on_response)(app_state, *message))
    }
}
