// Copyright 2024 the Xilem Authors
// SPDX-License-Identifier: Apache-2.0

use std::{future::Future, marker::PhantomData, rc::Rc};

use futures::{channel::oneshot, FutureExt};
use wasm_bindgen::UnwrapThrowExt;
use wasm_bindgen_futures::spawn_local;
use xilem_core::{MessageResult, Mut, NoElement, View, ViewId, ViewMarker};

use crate::{context::MessageThunk, DynMessage, Message, ViewCtx};

pub fn async_repeat<M, F, H, State, Action, Fut>(future: F, on_event: H) -> AsyncRepeat<F, H, M>
where
    F: Fn(AsyncRepeatProxy, ShutdownSignal) -> Fut + 'static,
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
        future,
        on_event,
        message: PhantomData,
    }
}

pub fn async_repeat_raw<M, F, H, State, Action, Fut>(future: F, on_event: H) -> AsyncRepeat<F, H, M>
where
    F: Fn(AsyncRepeatProxy, ShutdownSignal) -> Fut + 'static,
    Fut: Future<Output = ()> + 'static,
    H: Fn(&mut State, M) -> Action + 'static,
{
    AsyncRepeat {
        future,
        on_event,
        message: PhantomData,
    }
}

struct AbortHandle {
    abort_tx: oneshot::Sender<()>,
}

impl AbortHandle {
    fn abort(self) {
        let _ = self.abort_tx.send(());
    }
}

pub struct ShutdownSignal {
    shutdown_rx: oneshot::Receiver<()>,
}

impl ShutdownSignal {
    fn new() -> (Self, AbortHandle) {
        let (abort_tx, shutdown_rx) = oneshot::channel();
        (ShutdownSignal { shutdown_rx }, AbortHandle { abort_tx })
    }

    pub fn should_shutdown(&mut self) -> bool {
        match self.shutdown_rx.try_recv() {
            Ok(Some(())) | Err(oneshot::Canceled) => true,
            Ok(None) => false,
        }
    }

    pub fn into_future(self) -> impl Future<Output = ()> {
        self.shutdown_rx.map(|_| ())
    }
}

pub struct AsyncRepeat<F, H, M> {
    future: F,
    on_event: H,
    message: PhantomData<fn() -> M>,
}

pub struct AsyncRepeatState {
    abort_handle: Option<AbortHandle>,
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
    F: Fn(AsyncRepeatProxy, ShutdownSignal) -> Fut + 'static,
    Fut: Future<Output = ()> + 'static,
    H: Fn(&mut State, M) -> Action + 'static,
    M: Message,
{
    type Element = NoElement;

    type ViewState = AsyncRepeatState;

    fn build(&self, ctx: &mut ViewCtx) -> (Self::Element, Self::ViewState) {
        let thunk = ctx.message_thunk();
        let (shutdown_signal, abort_handle) = ShutdownSignal::new();
        let view_state = AsyncRepeatState {
            abort_handle: Some(abort_handle),
        };
        let proxy = AsyncRepeatProxy {
            thunk: Rc::new(thunk),
        };
        spawn_local((self.future)(proxy, shutdown_signal));
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
        let handle = view_state.abort_handle.take().unwrap_throw();
        handle.abort();
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
