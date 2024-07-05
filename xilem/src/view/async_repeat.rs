// Copyright 2024 the Xilem Authors
// SPDX-License-Identifier: Apache-2.0

use std::{future::Future, marker::PhantomData, sync::Arc};

use tokio::task::JoinHandle;
use xilem_core::{DynMessage, Message, MessageProxy, NoElement, View, ViewId, ViewPathTracker};

use crate::ViewCtx;

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
            .handle
            .spawn((self.future_future)(MessageProxy::new(proxy, path)));
        // TODO: Clearly this shouldn't be a label here
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
        _: &mut Self::ViewState,
        _: &mut ViewCtx,
        _: xilem_core::Mut<'_, Self::Element>,
    ) {
        // Nothing to do
        // TODO: Our state will be dropped, finishing the future
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
