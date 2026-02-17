// Copyright 2024 the Xilem Authors
// SPDX-License-Identifier: Apache-2.0

use std::any::{Any, TypeId};
use std::future::Future;
use std::marker::PhantomData;
use std::sync::Arc;

use tokio::sync::mpsc::{UnboundedReceiver, UnboundedSender};
use tokio::task::JoinHandle;

use crate::ViewCtx;
use crate::core::anymore::AnyDebug;
use crate::core::{
    MessageCtx, MessageProxy, MessageResult, Mut, NoElement, Resource, View, ViewId, ViewMarker,
    ViewPathTracker,
};

// TODO: Update generic variable names to be more .

/// Launch a task which will run until the view is no longer in the tree.
///
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
pub fn worker<F, H, M, S, V, State, Action, Fut>(
    init_future: F,
    store_sender: S,
    on_response: H,
) -> Worker<
    State,
    Action,
    F,
    H,
    M,
    impl Fn(&mut State, &mut Dummy, UnboundedSender<V>) + 'static,
    V,
    Dummy,
>
where
    F: Fn(MessageProxy<M>, UnboundedReceiver<V>) -> Fut,
    Fut: Future<Output = ()> + Send + 'static,
    S: Fn(&mut State, UnboundedSender<V>) + 'static,
    H: Fn(&mut State, M) -> Action + 'static,
    M: AnyDebug + Send + 'static,
    State: 'static,
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
        store_sender: move |state: &mut State,
                            _dummy: &mut Dummy,
                            sender: UnboundedSender<V>| {
            store_sender(state, sender);
        },
        on_response,
        message: PhantomData,
    }
}

/// A version of [`worker`] which can store its message sender in the environment.
///
/// This is an interim solution until we work out nicer
/// interactions with environment values in callbacks and the like.
pub fn env_worker<F, H, M, S, V, Res, State, Action, Fut>(
    init_future: F,
    store_sender: S,
    on_response: H,
) -> Worker<State, Action, F, H, M, S, V, Res>
where
    F: Fn(MessageProxy<M>, UnboundedReceiver<V>) -> Fut,
    Fut: Future<Output = ()> + Send + 'static,
    S: Fn(&mut State, &mut Res, UnboundedSender<V>),
    H: Fn(&mut State, M) -> Action + 'static,
    M: AnyDebug + Send + 'static,
    Res: Resource,
    State: 'static,
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

/// An internal struct used to make [`env_worker`]/[`worker`] work.
///
/// This is an interim solution until we design better ways for callbacks to interact with messages.
#[derive(Debug)]
#[doc(hidden)]
pub struct Dummy;

impl Resource for Dummy {}

/// Launch a worker which will run until the view is no longer in the tree.
///
/// This is [`worker`] without the capturing rules.
/// See `worker` for full documentation.
pub fn worker_raw<M, V, S, F, H, State, Action, Fut>(
    init_future: F,
    store_sender: S,
    on_response: H,
) -> Worker<
    State,
    Action,
    F,
    H,
    M,
    impl Fn(&mut State, &mut Dummy, UnboundedSender<V>) + 'static,
    V,
    Dummy,
>
where
    // TODO(DJMcNab): Accept app_state here
    F: Fn(MessageProxy<M>, UnboundedReceiver<V>) -> Fut,
    Fut: Future<Output = ()> + Send + 'static,
    S: Fn(&mut State, UnboundedSender<V>) + 'static,
    H: Fn(&mut State, M) -> Action + 'static,
    M: AnyDebug + Send + 'static,
    State: 'static,
{
    Worker {
        init_future,
        on_response,
        store_sender: move |state: &mut State,
                            _dummy: &mut Dummy,
                            sender: UnboundedSender<V>| {
            store_sender(state, sender);
        },
        message: PhantomData,
    }
}

/// The View type for [`worker`], [`env_worker`] and [`worker_raw`]. See its documentation for details.
pub struct Worker<State, Action, F, H, M, S, V, Res> {
    init_future: F,
    store_sender: S,
    on_response: H,
    message: PhantomData<fn(M, V, Res, State) -> Action>,
}

impl<State, Action, F, H, M, S, V, Res> ViewMarker for Worker<State, Action, F, H, M, S, V, Res> {}

impl<State, Action, F, H, M, Fut, S, V, Res> View<State, Action, ViewCtx>
    for Worker<State, Action, F, H, M, S, V, Res>
where
    Res: Resource,
    Action: 'static,
    F: Fn(MessageProxy<M>, UnboundedReceiver<V>) -> Fut + 'static,
    V: Send + 'static,
    Fut: Future<Output = ()> + Send + 'static,
    S: Fn(&mut State, &mut Res, UnboundedSender<V>) + 'static,
    H: Fn(&mut State, M) -> Action + 'static,
    M: AnyDebug + Send + 'static,
    State: 'static,
{
    type Element = NoElement;

    type ViewState = JoinHandle<()>;

    fn build(
        &self,
        ctx: &mut ViewCtx,
        app_state: &mut State,
    ) -> (Self::Element, Self::ViewState) {
        let path: Arc<[ViewId]> = ctx.view_path().into();

        let proxy = ctx.proxy();

        let (tx, rx) = tokio::sync::mpsc::unbounded_channel();
        let env = ctx.environment();
        if TypeId::of::<Res>() != TypeId::of::<Dummy>() {
            let pos = env.get_slot_for_type::<Res>();
            let Some(pos) = pos else {
                panic!(
                    // TODO: Track caller for this view?
                    "Xilem: Tried to get context for {}, but it hasn't been provided. Did you forget to wrap this view with `xilem_core::environment::provides`?",
                    core::any::type_name::<Res>()
                );
            };
            let slot = &mut env.slots[usize::try_from(pos).unwrap()];
            let Some(value) = slot.item.as_mut() else {
                panic!(
                    // TODO: Track caller for this view?
                    "Xilem: Tried to get context for {}, but it hasn't been `Provided`.",
                    core::any::type_name::<Res>()
                );
            };
            (self.store_sender)(
                app_state,
                value
                    .value
                    .downcast_mut::<Res>()
                    .expect("Environment's slots should have the correct types."),
                tx,
            );
        } else {
            // Hack: Use the same signature for both versions which need and don't need an environment.
            let value: &mut dyn Any = &mut Dummy;
            let res = value.downcast_mut::<Res>().expect("Has same type id.");
            (self.store_sender)(app_state, res, tx);
        }
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

    fn teardown(&self, handle: &mut Self::ViewState, _: &mut ViewCtx, _: Mut<'_, Self::Element>) {
        handle.abort();
    }

    fn message(
        &self,
        _: &mut Self::ViewState,
        message: &mut MessageCtx,
        _element: Mut<'_, Self::Element>,
        app_state: &mut State,
    ) -> MessageResult<Action> {
        debug_assert!(
            message.remaining_path().is_empty(),
            "id path should be empty in Task::message"
        );
        let message = message.take_message::<M>().unwrap();
        MessageResult::Action((self.on_response)(app_state, *message))
    }
}
