// Copyright 2024 the Xilem Authors
// SPDX-License-Identifier: Apache-2.0

#![expect(missing_docs, reason = "TODO - Document these items")]

use std::future::Future;
use std::marker::PhantomData;
use std::pin::Pin;
use std::sync::{Arc, Mutex};

use tokio::task::JoinHandle;

use crate::ViewCtx;
use crate::core::{
    AnyMessage, DynMessage, MessageProxy, MessageResult, Mut, NoElement, View, ViewId, ViewMarker,
    ViewPathTracker,
};

/// Launch a task which will run until the view is no longer in the tree.
///
/// If the view is removed from the tree and then later re-inserted the task will be
/// run again.
///
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
pub fn task<M, H, State, Action, Fut>(
    init_future: fn(MessageProxy<M>) -> Fut,
    on_event: H,
) -> Task<fn(MessageProxy<M>) -> Fut, H, M>
where
    Fut: Future<Output = ()> + Send + 'static,
    H: Fn(&mut State, M) -> Action + 'static,
    M: AnyMessage + 'static,
{
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
    M: AnyMessage + 'static,
{
    Task {
        init_future,
        on_event,
        message: PhantomData,
    }
}

/// Launch a task which will run until the view is no longer in the tree.
///
/// This task will only be run the first time this view is inserted in the tree. If the
/// view is stashed and then later re-inserted into the tree, the task will not be run again.
/// However, if the view is re-created and then inserted into the tree the task will be run again.
///
/// See [`task`] for full documentation.
pub fn task_raw_once<M, F, H, State, Action, Fut>(
    init_future: F,
    on_event: H,
) -> Task<impl Fn(MessageProxy<M>) -> Pin<Box<dyn Future<Output = ()> + Send + 'static>>, H, M>
where
    F: FnOnce(MessageProxy<M>) -> Fut,
    Fut: Future<Output = ()> + Send + 'static,
    H: Fn(&mut State, M) -> Action + 'static,
    M: AnyMessage + 'static,
{
    let init_future = Mutex::new(Some(init_future));
    let init_future = move |proxy| {
        let init_future = init_future.lock().unwrap().take();
        let future = init_future.map(|f| f(proxy));

        // We have to box the future to make it a nameable type
        Box::pin(async {
            if let Some(future) = future {
                future.await;
            }
        }) as Pin<Box<dyn Future<Output = ()> + Send + 'static>>
    };

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
    M: AnyMessage + 'static,
{
    type Element = NoElement;

    type ViewState = JoinHandle<()>;

    fn build(&self, ctx: &mut ViewCtx) -> (Self::Element, Self::ViewState) {
        let path: Arc<[ViewId]> = ctx.view_path().into();

        let proxy = ctx.proxy();
        let handle = ctx
            .runtime()
            .spawn((self.init_future)(MessageProxy::new(proxy, path)));
        (NoElement, handle)
    }

    fn rebuild(
        &self,
        _: &Self,
        _: &mut Self::ViewState,
        _: &mut ViewCtx,
        (): Mut<'_, Self::Element>,
    ) {
        // Nothing to do
    }

    fn teardown(
        &self,
        join_handle: &mut Self::ViewState,
        _: &mut ViewCtx,
        _: Mut<'_, Self::Element>,
    ) {
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
