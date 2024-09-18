// Copyright 2024 the Xilem Authors and the Druid Authors
// SPDX-License-Identifier: Apache-2.0

use std::{future::Future, marker::PhantomData};

use wasm_bindgen::{closure::Closure, JsCast, UnwrapThrowExt};
use wasm_bindgen_futures::spawn_local;
use xilem_core::{MessageResult, Mut, NoElement, View, ViewId, ViewMarker, ViewPathTracker};

use crate::{DynMessage, OptionalAction, ViewCtx};

/// Await a future returned by `init_future` invoked with the argument `data`, `callback` is called with the output of the future. `init_future` will be invoked again, when `data` changes. Use [`memoized_await`] for construction of this [`View`]
pub struct MemoizedAwait<State, Action, OA, InitFuture, Data, Callback, F, FOut> {
    init_future: InitFuture,
    data: Data,
    callback: Callback,
    debounce_ms: usize,
    reset_debounce_on_update: bool,
    phantom: PhantomData<fn() -> (State, Action, OA, F, FOut)>,
}

impl<State, Action, OA, InitFuture, Data, Callback, F, FOut>
    MemoizedAwait<State, Action, OA, InitFuture, Data, Callback, F, FOut>
where
    FOut: std::fmt::Debug + 'static,
    F: Future<Output = FOut> + 'static,
    InitFuture: Fn(&Data) -> F,
{
    /// Debounce the `init_future` function, when `data` updates,
    /// when `reset_debounce_on_update == false` then this throttles updates each `milliseconds`
    ///
    /// The default for this is `0`
    pub fn debounce_ms(mut self, milliseconds: usize) -> Self {
        self.debounce_ms = milliseconds;
        self
    }

    /// When `reset` is `true`, everytime `data` updates, the debounce timeout is cleared until `init_future` is invoked.
    /// This is only effective when `debounce > 0`
    ///
    /// The default for this is `true`
    pub fn reset_debounce_on_update(mut self, reset: bool) -> Self {
        self.reset_debounce_on_update = reset;
        self
    }

    fn init_future(&self, ctx: &mut ViewCtx, generation: u64) {
        ctx.with_id(ViewId::new(generation), |ctx| {
            let thunk = ctx.message_thunk();
            let future = (self.init_future)(&self.data);
            spawn_local(async move {
                thunk.push_message(MemoizedAwaitMessage::<FOut>::Output(future.await));
            });
        });
    }
}

/// Await a future returned by `init_future` invoked with the argument `data`, `callback` is called with the output of the resolved future. `init_future` will be invoked again, when `data` changes.
///
/// The update behavior can be controlled, by [`debounce_ms`](`MemoizedAwait::debounce_ms`) and [`reset_debounce_on_update`](`MemoizedAwait::reset_debounce_on_update`)
///
/// # Examples
///
/// ```
/// use xilem_web::{core::fork, concurrent::memoized_await, elements::html::div, interfaces::Element};
///
/// fn app_logic(state: &mut i32) -> impl Element<i32> {
///     fork(
///         div(*state),
///         memoized_await(
///             10,
///             |count| std::future::ready(*count),
///             |state, output| *state = output,
///         )
///     )
/// }
/// ```
pub fn memoized_await<State, Action, OA, InitFuture, Data, Callback, F, FOut>(
    data: Data,
    init_future: InitFuture,
    callback: Callback,
) -> MemoizedAwait<State, Action, OA, InitFuture, Data, Callback, F, FOut>
where
    State: 'static,
    Action: 'static,
    Data: PartialEq + 'static,
    FOut: std::fmt::Debug + 'static,
    F: Future<Output = FOut> + 'static,
    InitFuture: Fn(&Data) -> F + 'static,
    OA: OptionalAction<Action> + 'static,
    Callback: Fn(&mut State, FOut) -> OA + 'static,
{
    MemoizedAwait {
        init_future,
        data,
        callback,
        debounce_ms: 0,
        reset_debounce_on_update: true,
        phantom: PhantomData,
    }
}

#[derive(Default)]
pub struct MemoizedAwaitState {
    generation: u64,
    schedule_update: bool,
    // Closures are retained so they can be called by environment
    schedule_update_fn: Option<Closure<dyn FnMut()>>,
    schedule_update_timeout_handle: Option<i32>,
    update: bool,
}

impl MemoizedAwaitState {
    fn clear_update_timeout(&mut self) {
        if let Some(handle) = self.schedule_update_timeout_handle {
            web_sys::window()
                .unwrap_throw()
                .clear_timeout_with_handle(handle);
        }
        self.schedule_update_timeout_handle = None;
        self.schedule_update_fn = None;
    }

    fn reset_debounce_timeout_and_schedule_update<FOut: std::fmt::Debug + 'static>(
        &mut self,
        ctx: &mut ViewCtx,
        debounce_duration: usize,
    ) {
        ctx.with_id(ViewId::new(self.generation), |ctx| {
            self.clear_update_timeout();
            let thunk = ctx.message_thunk();
            let schedule_update_fn = Closure::new(move || {
                thunk.push_message(MemoizedAwaitMessage::<FOut>::ScheduleUpdate);
            });
            let handle = web_sys::window()
                .unwrap_throw()
                .set_timeout_with_callback_and_timeout_and_arguments_0(
                    schedule_update_fn.as_ref().unchecked_ref(),
                    debounce_duration.try_into().unwrap_throw(),
                )
                .unwrap_throw();
            self.schedule_update_fn = Some(schedule_update_fn);
            self.schedule_update_timeout_handle = Some(handle);
            self.schedule_update = true;
        });
    }
}

#[derive(Debug)]
enum MemoizedAwaitMessage<Output: std::fmt::Debug> {
    Output(Output),
    ScheduleUpdate,
}

impl<State, Action, OA, InitFuture, Data, CB, F, FOut> ViewMarker
    for MemoizedAwait<State, Action, OA, InitFuture, Data, CB, F, FOut>
{
}
impl<State, Action, InitFuture, F, FOut, Data, CB, OA> View<State, Action, ViewCtx, DynMessage>
    for MemoizedAwait<State, Action, OA, InitFuture, Data, CB, F, FOut>
where
    State: 'static,
    Action: 'static,
    OA: OptionalAction<Action> + 'static,
    InitFuture: Fn(&Data) -> F + 'static,
    FOut: std::fmt::Debug + 'static,
    Data: PartialEq + 'static,
    F: Future<Output = FOut> + 'static,
    CB: Fn(&mut State, FOut) -> OA + 'static,
{
    type Element = NoElement;

    type ViewState = MemoizedAwaitState;

    fn build(&self, ctx: &mut ViewCtx) -> (Self::Element, Self::ViewState) {
        let mut state = MemoizedAwaitState::default();

        if self.debounce_ms > 0 {
            state.reset_debounce_timeout_and_schedule_update::<FOut>(ctx, self.debounce_ms);
        } else {
            self.init_future(ctx, state.generation);
        }

        (NoElement, state)
    }

    fn rebuild<'el>(
        &self,
        prev: &Self,
        view_state: &mut Self::ViewState,
        ctx: &mut ViewCtx,
        (): Mut<'el, Self::Element>,
    ) -> Mut<'el, Self::Element> {
        let debounce_has_changed_and_update_is_scheduled = view_state.schedule_update
            && (prev.reset_debounce_on_update != self.reset_debounce_on_update
                || prev.debounce_ms != self.debounce_ms);

        if debounce_has_changed_and_update_is_scheduled {
            if self.debounce_ms == 0 {
                if view_state.schedule_update_timeout_handle.is_some() {
                    view_state.clear_update_timeout();
                    view_state.schedule_update = false;
                    view_state.update = true;
                }
            } else {
                view_state
                    .reset_debounce_timeout_and_schedule_update::<FOut>(ctx, self.debounce_ms);
                return; // avoid update below, as it's already scheduled
            }
        }

        if view_state.update
            || (prev.data != self.data
                && (!view_state.schedule_update || self.reset_debounce_on_update))
        {
            if !view_state.update && self.debounce_ms > 0 {
                view_state
                    .reset_debounce_timeout_and_schedule_update::<FOut>(ctx, self.debounce_ms);
            } else {
                // no debounce
                view_state.generation += 1;
                view_state.update = false;
                self.init_future(ctx, view_state.generation);
            }
        }
    }

    fn teardown(&self, state: &mut Self::ViewState, _: &mut ViewCtx, (): Mut<'_, Self::Element>) {
        state.clear_update_timeout();
    }

    fn message(
        &self,
        view_state: &mut Self::ViewState,
        id_path: &[ViewId],
        message: DynMessage,
        app_state: &mut State,
    ) -> MessageResult<Action, DynMessage> {
        assert_eq!(id_path.len(), 1);
        if id_path[0].routing_id() == view_state.generation {
            match *message.downcast().unwrap_throw() {
                MemoizedAwaitMessage::Output(future_output) => {
                    match (self.callback)(app_state, future_output).action() {
                        Some(action) => MessageResult::Action(action),
                        None => MessageResult::Nop,
                    }
                }
                MemoizedAwaitMessage::ScheduleUpdate => {
                    view_state.update = true;
                    view_state.schedule_update = false;
                    MessageResult::RequestRebuild
                }
            }
        } else {
            MessageResult::Stale(message)
        }
    }
}
