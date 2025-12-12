// Copyright 2024 the Xilem Authors
// SPDX-License-Identifier: Apache-2.0

#![expect(missing_docs, reason = "TODO - Document these items")]

use std::future::Future;
use std::marker::PhantomData;
use std::sync::Arc;

use tokio::task::JoinHandle;

use crate::ViewCtx;
use crate::core::anymore::AnyDebug;
use crate::core::{
    Arg, MessageCtx, MessageProxy, MessageResult, Mut, NoElement, View, ViewArgument, ViewId,
    ViewMarker, ViewPathTracker,
};

/// Launch a task which will run until the view is no longer in the tree.
/// `init_future` is given a [`MessageProxy`], which it will store in the future it returns.
/// This `MessageProxy` can be used to send a message to `on_event`, which can then update
/// the app's state.
///
/// For example, this can be used with the time functions in [`tokio::time`].
///
/// Note that this task will not be updated if the view is rebuilt, so `init_future`
/// cannot capture.
// TODO: More thorough documentation.
/// See [`run_once`](crate::core::run_once) for details.
pub fn task<M, F, H, State, Action, Fut>(
    init_future: F,
    on_event: H,
) -> Task<State, Action, F, H, M>
where
    F: Fn(MessageProxy<M>, Arg<'_, State>) -> Fut + 'static,
    Fut: Future<Output = ()> + Send + 'static,
    H: Fn(Arg<'_, State>, M) -> Action + 'static,
    M: AnyDebug + Send + 'static,
    State: ViewArgument,
    Action: 'static,
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
    .check_impl_view()
}

/// Launch a task which will run until the view is no longer in the tree.
///
/// This is [`task`] without the capturing rules.
/// See `task` for full documentation.
pub fn task_raw<M, F, H, State, Action, Fut>(
    init_future: F,
    on_event: H,
) -> Task<State, Action, F, H, M>
where
    F: Fn(MessageProxy<M>, Arg<'_, State>) -> Fut,
    Fut: Future<Output = ()> + Send + 'static,
    H: Fn(Arg<'_, State>, M) -> Action + 'static,
    M: AnyDebug + Send + 'static,
    State: ViewArgument,
{
    Task {
        init_future,
        on_event,
        message: PhantomData,
    }
}

pub struct Task<State, Action, F, H, M> {
    init_future: F,
    on_event: H,
    message: PhantomData<fn(State) -> (Action, M)>,
}

impl<State, Action, F, H, M> ViewMarker for Task<State, Action, F, H, M> {}
impl<State, Action, F, H, M, Fut> View<State, Action, ViewCtx> for Task<State, Action, F, H, M>
where
    State: ViewArgument,
    Action: 'static,
    F: Fn(MessageProxy<M>, Arg<'_, State>) -> Fut + 'static,
    Fut: Future<Output = ()> + Send + 'static,
    H: Fn(Arg<'_, State>, M) -> Action + 'static,
    M: AnyDebug + Send + 'static,
{
    type Element = NoElement;

    type ViewState = JoinHandle<()>;

    fn build(&self, ctx: &mut ViewCtx, state: Arg<'_, State>) -> (Self::Element, Self::ViewState) {
        let path: Arc<[ViewId]> = ctx.view_path().into();

        let proxy = ctx.proxy();
        let handle = ctx
            .runtime()
            .spawn((self.init_future)(MessageProxy::new(proxy, path), state));
        (NoElement, handle)
    }

    fn rebuild(
        &self,
        _: &Self,
        _: &mut Self::ViewState,
        _: &mut ViewCtx,
        (): Mut<'_, Self::Element>,
        _: Arg<'_, State>,
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
        message: &mut MessageCtx,
        _element: Mut<'_, Self::Element>,
        app_state: Arg<'_, State>,
    ) -> MessageResult<Action> {
        debug_assert!(
            message.remaining_path().is_empty(),
            "id path should be empty in Task::message"
        );
        let message = message.take_message::<M>().unwrap();
        MessageResult::Action((self.on_event)(app_state, *message))
    }
}
