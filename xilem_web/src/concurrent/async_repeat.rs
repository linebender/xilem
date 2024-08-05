// Copyright 2024 the Xilem Authors
// SPDX-License-Identifier: Apache-2.0

use std::{future::Future, marker::PhantomData, rc::Rc};

use futures::channel::oneshot;
use wasm_bindgen_futures::spawn_local;
use xilem_core::{MessageResult, Mut, NoElement, View, ViewId, ViewMarker};

use crate::{context::MessageThunk, DynMessage, Message, ViewCtx};

pub fn async_repeat<M, F, H, State, Action, Fut>(
    future_future: F,
    on_event: H,
) -> AsyncRepeat<F, H, M>
where
    F: Fn(AsyncRepeatProxy, oneshot::Receiver<()>) -> Fut + 'static,
    Fut: Future<Output = ()> + 'static,
    H: Fn(&mut State, M) -> Action + 'static,
    M: Message,
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

pub fn async_repeat_raw<M, F, H, State, Action, Fut>(
    future_future: F,
    on_event: H,
) -> AsyncRepeat<F, H, M>
where
    F: Fn(MessageThunk) -> Fut + 'static,
    Fut: Future<Output = ()> + 'static,
    H: Fn(&mut State, M) -> Action + 'static,
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

pub struct AsyncRepeatState {
    abort_tx: Option<oneshot::Sender<()>>,
}

pub struct AsyncRepeatProxy {
    thunk: Rc<MessageThunk>,
}

impl AsyncRepeatProxy {
    pub fn send_message<M>(&self, message: M)
    where
        M: Message,
    {
        let thunk = Rc::clone(&self.thunk);
        spawn_local(async move {
            thunk.push_message(message);
        });
    }
}

impl<F, H, M> ViewMarker for AsyncRepeat<F, H, M> {}

impl<State, Action, F, H, M, Fut> View<State, Action, ViewCtx, DynMessage> for AsyncRepeat<F, H, M>
where
    State: 'static,
    Action: 'static,
    F: Fn(AsyncRepeatProxy, oneshot::Receiver<()>) -> Fut + 'static,
    Fut: Future<Output = ()> + 'static,
    H: Fn(&mut State, M) -> Action + 'static,
    M: Message,
{
    type Element = NoElement;

    type ViewState = AsyncRepeatState;

    fn build(&self, ctx: &mut ViewCtx) -> (Self::Element, Self::ViewState) {
        let thunk = ctx.message_thunk();
        let (abort_tx, abort_rx) = oneshot::channel();
        let view_state = AsyncRepeatState {
            abort_tx: Some(abort_tx),
        };
        let proxy = AsyncRepeatProxy {
            thunk: Rc::new(thunk),
        };
        spawn_local((self.future_future)(proxy, abort_rx));
        (NoElement, view_state)
    }

    fn rebuild<'el>(
        &self,
        _: &Self,
        _: &mut Self::ViewState,
        _: &mut ViewCtx,
        (): Mut<'el, Self::Element>,
    ) -> Mut<'el, Self::Element> {
        // Nothing to do
    }

    fn teardown(
        &self,
        view_state: &mut Self::ViewState,
        _: &mut ViewCtx,
        _: Mut<'_, Self::Element>,
    ) {
        let Some(tx) = view_state.abort_tx.take() else {
            // TODO: Is that even allowed to happen?
            return;
        };
        let _ = tx.send(());
    }

    fn message(
        &self,
        _: &mut Self::ViewState,
        id_path: &[ViewId],
        message: DynMessage,
        app_state: &mut State,
    ) -> MessageResult<Action, DynMessage> {
        debug_assert!(
            id_path.is_empty(),
            "id path should be empty in AsyncRepeat::message"
        );
        let message = message.downcast::<M>().unwrap();
        MessageResult::Action((self.on_event)(app_state, *message))
    }
}
