// Copyright 2024 the Xilem Authors
// SPDX-License-Identifier: Apache-2.0

#![expect(missing_docs, reason = "TODO - Document these items")]

use std::future::Future;
use std::marker::PhantomData;
use std::sync::Arc;

use tokio::sync::mpsc::{UnboundedReceiver, UnboundedSender};
use tokio::task::JoinHandle;

use crate::ViewCtx;
use crate::core::{
    AnyMessage, DynMessage, MessageProxy, MessageResult, Mut, NoElement, View, ViewId, ViewMarker,
    ViewPathTracker,
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
pub fn worker<F, H, M, S, V, State, Action, Fut>(
    init_future: F,
    store_sender: S,
    on_response: H,
) -> Worker<F, H, M, S, V>
where
    F: Fn(MessageProxy<M>, UnboundedReceiver<V>) -> Fut,
    Fut: Future<Output = ()> + Send + 'static,
    S: Fn(&mut State, UnboundedSender<V>),
    H: Fn(&mut State, M) -> Action + 'static,
    M: AnyMessage + 'static,
{
    const {
        assert!(
            size_of::<F>() == 0,
            "`worker` will not be ran again when its captured variables are updated.\n\
            To ignore this warning, use `worker_raw`."
        );
    };
    Worker {
        init_future,
        store_sender,
        on_response,
        message: PhantomData,
    }
}

/// Launch a worker which will run until the view is no longer in the tree.
///
/// This is [`worker`] without the capturing rules.
/// See `worker` for full documentation.
pub fn worker_raw<M, V, S, F, H, State, Action, Fut>(
    init_future: F,
    store_sender: S,
    on_response: H,
) -> Worker<F, H, M, S, V>
where
    // TODO(DJMcNab): Accept app_state here
    F: Fn(MessageProxy<M>, UnboundedReceiver<V>) -> Fut,
    Fut: Future<Output = ()> + Send + 'static,
    S: Fn(&mut State, UnboundedSender<V>),
    H: Fn(&mut State, M) -> Action + 'static,
    M: AnyMessage + 'static,
{
    Worker {
        init_future,
        on_response,
        store_sender,
        message: PhantomData,
    }
}

pub struct Worker<F, H, M, S, V> {
    init_future: F,
    store_sender: S,
    on_response: H,
    message: PhantomData<fn(M, V)>,
}

impl<F, H, M, S, V> ViewMarker for Worker<F, H, M, S, V> {}

impl<State, Action, F, H, M, Fut, S, V> View<State, Action, ViewCtx> for Worker<F, H, M, S, V>
where
    F: Fn(MessageProxy<M>, UnboundedReceiver<V>) -> Fut + 'static,
    V: Send + 'static,
    Fut: Future<Output = ()> + Send + 'static,
    S: Fn(&mut State, UnboundedSender<V>) + 'static,
    H: Fn(&mut State, M) -> Action + 'static,
    M: AnyMessage + 'static,
{
    type Element = NoElement;

    type ViewState = JoinHandle<()>;

    fn build(&self, ctx: &mut ViewCtx, app_state: &mut State) -> (Self::Element, Self::ViewState) {
        let path: Arc<[ViewId]> = ctx.view_path().into();

        let proxy = ctx.proxy();

        let (tx, rx) = tokio::sync::mpsc::unbounded_channel();
        (self.store_sender)(app_state, tx);
        let handle = ctx
            .runtime()
            .spawn((self.init_future)(MessageProxy::new(proxy, path), rx));
        (NoElement, handle)
    }

    fn rebuild(
        &self,
        _prev: &Self,
        _view_state: &mut Self::ViewState,
        _: &mut ViewCtx,
        (): Mut<'_, Self::Element>,
        _: &mut State,
    ) {
    }

    fn teardown(
        &self,
        handle: &mut Self::ViewState,
        _: &mut ViewCtx,
        _: Mut<'_, Self::Element>,
        _: &mut State,
    ) {
        handle.abort();
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
