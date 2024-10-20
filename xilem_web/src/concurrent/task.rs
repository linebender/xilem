// Copyright 2024 the Xilem Authors
// SPDX-License-Identifier: Apache-2.0

use std::{future::Future, marker::PhantomData, rc::Rc};

use crate::{
    context::MessageThunk,
    core::{MessageResult, Mut, NoElement, View, ViewId, ViewMarker},
    DynMessage, Message, ViewCtx,
};
use futures::{channel::oneshot, FutureExt};
use wasm_bindgen::UnwrapThrowExt;
use wasm_bindgen_futures::spawn_local;

/// Spawn an async task to update state asynchronously
///
/// The `init_future` function is given a [`TaskProxy`] and a [`ShutdownSignal`].
/// The `TaskProxy` can be used to send a message to `on_event`, which can then update
/// the app's state.
/// The `ShutdownSignal` can be used to detect whether the view has disappeared and
/// a shutdown request has been made (because a future cannot be canceled from the outside).
///
/// Note that this task will not be updated if the view is rebuilt, so `init_future`
/// cannot capture.
pub fn task<M, F, H, State, Action, Fut>(init_future: F, on_event: H) -> Task<F, H, M>
where
    F: Fn(TaskProxy, ShutdownSignal) -> Fut + 'static,
    Fut: Future<Output = ()> + 'static,
    H: Fn(&mut State, M) -> Action + 'static,
    M: Message,
{
    const {
        assert!(
            core::mem::size_of::<F>() == 0,
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

/// Spawn a future.
///
/// This is [`task`] without the capturing rules.
/// See `task` for full documentation.
pub fn task_raw<M, F, H, State, Action, Fut>(init_future: F, on_event: H) -> Task<F, H, M>
where
    F: Fn(TaskProxy, ShutdownSignal) -> Fut + 'static,
    Fut: Future<Output = ()> + 'static,
    H: Fn(&mut State, M) -> Action + 'static,
{
    Task {
        init_future,
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

    /// Detect whether the view has disappeared and
    /// a shutdown request has been made.
    pub fn should_shutdown(&mut self) -> bool {
        match self.shutdown_rx.try_recv() {
            Ok(Some(())) | Err(oneshot::Canceled) => true,
            Ok(None) => false,
        }
    }

    /// Transform the signal into a future
    /// that resolves if a shutdown request has been made.
    pub fn into_future(self) -> impl Future<Output = ()> {
        self.shutdown_rx.map(|_| ())
    }
}

pub struct Task<F, H, M> {
    init_future: F,
    on_event: H,
    message: PhantomData<fn() -> M>,
}

#[allow(unnameable_types)] // reason: Implementation detail, public because of trait visibility rules
pub struct TaskState {
    abort_handle: Option<AbortHandle>,
}

pub struct TaskProxy {
    thunk: Rc<MessageThunk>,
}

impl TaskProxy {
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

impl<F, H, M> ViewMarker for Task<F, H, M> {}

impl<State, Action, F, H, M, Fut> View<State, Action, ViewCtx, DynMessage> for Task<F, H, M>
where
    State: 'static,
    Action: 'static,
    F: Fn(TaskProxy, ShutdownSignal) -> Fut + 'static,
    Fut: Future<Output = ()> + 'static,
    H: Fn(&mut State, M) -> Action + 'static,
    M: Message,
{
    type Element = NoElement;

    type ViewState = TaskState;

    fn build(&self, ctx: &mut ViewCtx) -> (Self::Element, Self::ViewState) {
        let thunk = ctx.message_thunk();
        let (shutdown_signal, abort_handle) = ShutdownSignal::new();
        let view_state = TaskState {
            abort_handle: Some(abort_handle),
        };
        let proxy = TaskProxy {
            thunk: Rc::new(thunk),
        };
        spawn_local((self.init_future)(proxy, shutdown_signal));
        (NoElement, view_state)
    }

    fn rebuild(&self, _: &Self, _: &mut Self::ViewState, _: &mut ViewCtx, (): Mut<Self::Element>) {
        // Nothing to do
    }

    fn teardown(&self, view_state: &mut Self::ViewState, _: &mut ViewCtx, _: Mut<Self::Element>) {
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
