// Copyright 2024 the Xilem Authors
// SPDX-License-Identifier: Apache-2.0

use std::{future::Future, marker::PhantomData, sync::Arc};

use tokio::task::JoinHandle;
use xilem_core::{
    DynMessage, Message, MessageProxy, NoElement, View, ViewId, ViewMarker, ViewPathTracker,
};

use crate::ViewCtx;

/// Launch a task which will run until the view is no longer in the tree.
/// `future_future` is given a [`MessageProxy`], which it will store in the future it returns.
/// This `MessageProxy` can be used to send a message to `on_event`, which can then update
/// the app's state.
///
/// For exampe, this can be used with the time functions in [`crate::tokio::time`].
///
/// Note that this task will not be updated if the view is rebuilt, so `future_future`
/// cannot capture.
// TODO: More thorough documentation.
/// See [`run_once`](crate::core::run_once) for details.
pub fn async_repeat<M, F, H, State, Action, Fut>(
    future_future: F,
    on_event: H,
) -> AsyncRepeat<F, H, M>
where
    F: Fn(MessageProxy<M>) -> Fut,
    Fut: Future<Output = ()> + Send + 'static,
    H: Fn(&mut State, M) -> Action + 'static,
    M: Message + 'static,
{
    const {
        assert!(
            core::mem::size_of::<F>() == 0,
            "`async_repeat` will not be ran again when its captured variables are updated.\n\
            To ignore this warning, use `async_repeat_raw`."
        );
    };
    AsyncRepeat {
        future_future,
        on_event,
        message: PhantomData,
    }
}

/// Launch a task which will run until the view is no longer in the tree.
///
/// This is [`async_repeat`] without the capturing rules.
/// See `async_repeat` for full documentation.
pub fn async_repeat_raw<M, F, H, State, Action, Fut>(
    future_future: F,
    on_event: H,
) -> AsyncRepeat<F, H, M>
where
    F: Fn(MessageProxy<M>) -> Fut,
    Fut: Future<Output = ()> + Send + 'static,
    H: Fn(&mut State, M) -> Action + 'static,
    M: Message + 'static,
{
    AsyncRepeat {
        future_future,
        on_event,
        message: PhantomData,
    }
}

pub struct AsyncRepeat<F, H, M> {
    future_future: F,
    on_event: H,
    message: PhantomData<fn() -> M>,
}

impl<F, H, M> ViewMarker for AsyncRepeat<F, H, M> {}
impl<State, Action, F, H, M, Fut> View<State, Action, ViewCtx> for AsyncRepeat<F, H, M>
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
            .spawn((self.future_future)(MessageProxy::new(proxy, path)));
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
        join_handle.abort();
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
