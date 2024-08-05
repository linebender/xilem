// Copyright 2024 the Xilem Authors and the Druid Authors
// SPDX-License-Identifier: Apache-2.0

use std::marker::PhantomData;

use wasm_bindgen::{closure::Closure, JsCast, UnwrapThrowExt};
use xilem_core::{MessageResult, Mut, NoElement, View, ViewId, ViewMarker};

use crate::{DynMessage, OptionalAction, ViewCtx};

/// Start an interval which invokes `callback` every `ms` milliseconds
pub struct Interval<Callback, State, Action> {
    ms: u32,
    callback: Callback,
    phantom: PhantomData<fn() -> (State, Action)>,
}

/// Start an interval which invokes `callback` every `ms` milliseconds
///
/// Currently, when `ms` changes, the previous interval is cleared, and starts with the new interval.
/// This default behavior may change in the future, and may even be configurable.
///
/// # Examples
///
/// ```
/// use xilem_web::{core::fork, concurrent::interval, elements::html::div, interfaces::Element};
///
/// fn timer(seconds: &mut u32) -> impl Element<u32> {
///     fork(
///         div(format!("{seconds} seconds have passed, since creating this view")),
///         interval(
///             1000, // in ms, when this changes, the interval is reset
///             |seconds: &mut u32| *seconds += 1,
///         )
///     )
/// }
/// ```
///
/// # Panics
///
/// While `ms` is a `u32`, `setInterval` actually requires this to be a `i32`, so values above `2147483647` lead to a panic.
/// See <https://developer.mozilla.org/en-US/docs/Web/API/setInterval#sect2> for more details.
pub fn interval<State, Action, OA, Callback>(
    ms: u32,
    callback: Callback,
) -> Interval<Callback, State, Action>
where
    State: 'static,
    Action: 'static,
    OA: OptionalAction<Action> + 'static,
    Callback: Fn(&mut State) -> OA + 'static,
{
    Interval {
        ms,
        callback,
        phantom: PhantomData,
    }
}

pub struct IntervalState {
    // Closures are retained so they can be called by environment
    interval_fn: Closure<dyn FnMut()>,
    interval_handle: i32,
}

fn start_interval(callback: &Closure<dyn FnMut()>, ms: u32) -> i32 {
    web_sys::window()
        .unwrap_throw()
        .set_interval_with_callback_and_timeout_and_arguments_0(
            callback.as_ref().unchecked_ref(),
            ms.try_into().expect_throw(
                "`setInterval` requires this to be an `i32`,\
                 which is why values above `2147483647` are not possible,\
                 see https://developer.mozilla.org/en-US/docs/Web/API/setInterval#sect2 \
                 for more details",
            ),
        )
        .unwrap_throw()
}

fn clear_interval(handle: i32) {
    web_sys::window()
        .unwrap_throw()
        .clear_interval_with_handle(handle);
}

impl<Callback, State, Action> ViewMarker for Interval<Callback, State, Action> {}

impl<State, Action, Callback, OA> View<State, Action, ViewCtx, DynMessage>
    for Interval<Callback, State, Action>
where
    State: 'static,
    Action: 'static,
    OA: OptionalAction<Action> + 'static,
    Callback: Fn(&mut State) -> OA + 'static,
{
    type Element = NoElement;

    type ViewState = IntervalState;

    fn build(&self, ctx: &mut ViewCtx) -> (Self::Element, Self::ViewState) {
        let thunk = ctx.message_thunk();
        let interval_fn = Closure::new(move || thunk.push_message(()));
        let state = IntervalState {
            interval_handle: start_interval(&interval_fn, self.ms),
            interval_fn,
        };

        (NoElement, state)
    }

    fn rebuild<'el>(
        &self,
        prev: &Self,
        view_state: &mut Self::ViewState,
        _: &mut ViewCtx,
        (): Mut<'el, Self::Element>,
    ) -> Mut<'el, Self::Element> {
        if prev.ms != self.ms {
            clear_interval(view_state.interval_handle);
            view_state.interval_handle = start_interval(&view_state.interval_fn, self.ms);
        }
    }

    fn teardown(
        &self,
        view_state: &mut Self::ViewState,
        _: &mut ViewCtx,
        _: Mut<'_, Self::Element>,
    ) {
        clear_interval(view_state.interval_handle);
    }

    fn message(
        &self,
        _: &mut Self::ViewState,
        id_path: &[ViewId],
        message: DynMessage,
        app_state: &mut State,
    ) -> MessageResult<Action, DynMessage> {
        debug_assert!(id_path.is_empty());
        message.downcast::<()>().unwrap_throw();
        match (self.callback)(app_state).action() {
            Some(action) => MessageResult::Action(action),
            None => MessageResult::Nop,
        }
    }
}
