// Copyright 2024 the Xilem Authors
// SPDX-License-Identifier: Apache-2.0

use std::future::Future;
use std::marker::PhantomData;
use std::sync::Arc;

use tokio::task::JoinHandle;

use crate::core::{
    DynMessage, Message, MessageProxy, MessageResult, Mut, NoElement, View, ViewId, ViewMarker,
    ViewPathTracker,
};
use crate::ViewCtx;

/// Launch a task which will run until the view is no longer in the tree.
/// `init_future` is given a [`MessageProxy`], which it will store in the future it returns.
/// This `MessageProxy` can be used to send a message to `on_event`, which can then update
/// the app's state.
///
/// For exampe, this can be used with the time functions in [`crate::tokio::time`].
///
/// Note that this task will not be updated if the view is rebuilt, so `init_future`
/// cannot capture.
// TODO: More thorough documentation.
/// See [`run_once`](crate::core::run_once) for details.
pub fn task<M, F, H, State, Action, Fut>(init_future: F, on_event: H) -> Task<F, H, M>
where
    F: Fn(MessageProxy<M>) -> Fut,
    Fut: Future<Output = ()> + Send + 'static,
    H: Fn(&mut State, M) -> Action + 'static,
    M: Message + 'static,
{
    const {
        assert!(
            size_of::<F>() == 0,
            "`task` will not be ran again when its captured variables are updated.\n\
            To ignore this warning, use `task_raw`."
        );
    };
    Task {
        init_future,
        on_event,
        message: PhantomData,
    }
}

/// Launch a task which will run until the view is no longer in the tree.
///
/// This is [`task`] without the capturing rules.
/// See `task` for full documentation.
pub fn task_raw<M, F, H, State, Action, Fut>(init_future: F, on_event: H) -> Task<F, H, M>
where
    F: Fn(MessageProxy<M>) -> Fut,
    Fut: Future<Output = ()> + Send + 'static,
    H: Fn(&mut State, M) -> Action + 'static,
    M: Message + 'static,
{
    Task {
        init_future,
        on_event,
        message: PhantomData,
    }
}

pub struct Task<F, H, M> {
    init_future: F,
    on_event: H,
    message: PhantomData<fn() -> M>,
}

impl<F, H, M> ViewMarker for Task<F, H, M> {}
impl<State, Action, F, H, M, Fut> View<State, Action, ViewCtx> for Task<F, H, M>
where
    F: Fn(MessageProxy<M>) -> Fut + 'static,
    Fut: Future<Output = ()> + Send + 'static,
    H: Fn(&mut State, M) -> Action + 'static,
    M: Message + 'static,
{
    type Element = NoElement;

    type ViewState = JoinHandle<()>;

    fn build(&self, ctx: &mut ViewCtx) -> (Self::Element, Self::ViewState) {
        let path: Arc<[ViewId]> = ctx.view_path().into();

        let proxy = ctx.proxy.clone();
        let handle = ctx
            .runtime()
            .spawn((self.init_future)(MessageProxy::new(proxy, path)));
        (NoElement, handle)
    }

    fn rebuild(&self, _: &Self, _: &mut Self::ViewState, _: &mut ViewCtx, (): Mut<Self::Element>) {
        // Nothing to do
    }

    fn teardown(&self, join_handle: &mut Self::ViewState, _: &mut ViewCtx, _: Mut<Self::Element>) {
        join_handle.abort();
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
        MessageResult::Action((self.on_event)(app_state, *message))
    }
}
