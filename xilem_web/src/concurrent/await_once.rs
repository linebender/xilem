// Copyright 2024 the Xilem Authors and the Druid Authors
// SPDX-License-Identifier: Apache-2.0

use std::{future::Future, marker::PhantomData};

use wasm_bindgen::UnwrapThrowExt;
use wasm_bindgen_futures::spawn_local;
use xilem_core::{MessageResult, Mut, NoElement, View, ViewId};

use crate::{DynMessage, OptionalAction, ViewCtx};

/// Await a future returned by `init_future`, `callback` is called with the output of the future. `init_future` will only be invoked once. Use [`await_once`] for construction of this [`View`]
pub struct AwaitOnce<InitFuture, Callback, State, Action> {
    init_future: InitFuture,
    callback: Callback,
    phantom: PhantomData<fn() -> (State, Action)>,
}

/// Await a future returned by `init_future`, `callback` is called with the output of the future. `init_future` will only be invoked once.
///
/// # Examples
///
/// ```
/// use xilem_web::{core::fork, concurrent::await_once, elements::html::div, interfaces::Element};
///
/// fn app_logic(state: &mut i32) -> impl Element<i32> {
///     fork(
///         div(*state),
///         await_once(
///             |_state: &mut i32| std::future::ready(42),
///             |state: &mut i32, meaning_of_life| *state = meaning_of_life,
///         )
///     )
/// }
/// ```
pub fn await_once<State, Action, FOut, F, InitFuture, OA, Callback>(
    init_future: InitFuture,
    callback: Callback,
) -> AwaitOnce<InitFuture, Callback, State, Action>
where
    State: 'static,
    Action: 'static,
    FOut: std::fmt::Debug + 'static,
    F: Future<Output = FOut> + 'static,
    InitFuture: Fn(&mut State) -> F + 'static,
    OA: OptionalAction<Action> + 'static,
    Callback: Fn(&mut State, FOut) -> OA + 'static,
{
    AwaitOnce {
        init_future,
        callback,
        phantom: PhantomData,
    }
}

pub struct AwaitOnceState<F> {
    future: Option<F>,
}

impl<State, Action, InitFuture, F, FOut, Callback, OA> View<State, Action, ViewCtx, DynMessage>
    for AwaitOnce<InitFuture, Callback, State, Action>
where
    State: 'static,
    Action: 'static,
    FOut: std::fmt::Debug + 'static,
    F: Future<Output = FOut> + 'static,
    InitFuture: Fn(&mut State) -> F + 'static,
    OA: OptionalAction<Action> + 'static,
    Callback: Fn(&mut State, FOut) -> OA + 'static,
{
    type Element = NoElement;

    type ViewState = AwaitOnceState<F>;

    fn build(&self, ctx: &mut ViewCtx) -> (Self::Element, Self::ViewState) {
        let thunk = ctx.message_thunk();
        // we can't directly push the initial message, as this would be executed directly (not in the next microtick), which in turn means that the already mutably borrowed `App` would be borrowed again.
        // So we have to delay this with a spawn_local
        spawn_local(async move { thunk.push_message(None::<FOut>) });
        (NoElement, AwaitOnceState { future: None })
    }

    fn rebuild<'el>(
        &self,
        _prev: &Self,
        view_state: &mut Self::ViewState,
        ctx: &mut ViewCtx,
        (): Mut<'el, Self::Element>,
    ) -> Mut<'el, Self::Element> {
        if let Some(future) = view_state.future.take() {
            let thunk = ctx.message_thunk();
            spawn_local(async move { thunk.push_message(Some(future.await)) });
        }
    }

    fn teardown(&self, _: &mut Self::ViewState, _: &mut ViewCtx, _: Mut<'_, Self::Element>) {}

    fn message(
        &self,
        view_state: &mut Self::ViewState,
        id_path: &[ViewId],
        message: DynMessage,
        app_state: &mut State,
    ) -> MessageResult<Action, DynMessage> {
        assert!(id_path.is_empty()); // `debug_assert!` instead? to save some bytes in the release binary?
        match *message.downcast().unwrap_throw() {
            Some(future_output) => match (self.callback)(app_state, future_output).action() {
                Some(action) => MessageResult::Action(action),
                None => MessageResult::Nop,
            },
            // Initial trigger in build, invoke the init_future and spawn it in `View::rebuild`
            None => {
                view_state.future = Some((self.init_future)(app_state));
                MessageResult::RequestRebuild
            }
        }
    }
}
