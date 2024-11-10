// Copyright 2023 the Xilem Authors
// SPDX-License-Identifier: Apache-2.0

//! Interactivity with pointer events.

use crate::{
    core::{MessageResult, Mut, View, ViewId, ViewMarker, ViewPathTracker},
    interfaces::Element,
    DomView, DynMessage, ViewCtx,
};
use peniko::kurbo::Point;
use std::marker::PhantomData;
use wasm_bindgen::{prelude::Closure, throw_str, JsCast, UnwrapThrowExt};
use web_sys::PointerEvent;

/// Use a distinctive number here, to be able to catch bugs.
/// In case the generational-id view path in `View::Message` lead to a wrong view
const POINTER_VIEW_ID: ViewId = ViewId::new(0x1234_5014);

/// A view that allows stateful handling of [`PointerEvent`]s with [`PointerMsg`]
///
/// See [`Element::pointer`] for more details how to use this view.
pub struct Pointer<V, T, A, F> {
    child: V,
    callback: F,
    phantom: PhantomData<fn() -> (T, A)>,
}

#[allow(
    unnameable_types,
    reason = "Implementation detail, public because of trait visibility rules"
)]
pub struct PointerState<S> {
    // reason: Closures are retained so they can be called by environment
    #[allow(unused)]
    down_closure: Closure<dyn FnMut(PointerEvent)>,
    #[allow(unused)]
    move_closure: Closure<dyn FnMut(PointerEvent)>,
    #[allow(unused)]
    up_closure: Closure<dyn FnMut(PointerEvent)>,
    child_state: S,
}

#[derive(Debug)]
/// A message representing a pointer event.
pub enum PointerMsg {
    Down(PointerDetails),
    Move(PointerDetails),
    Up(PointerDetails),
}

impl PointerMsg {
    pub fn position(&self) -> Point {
        match self {
            PointerMsg::Down(p) | PointerMsg::Move(p) | PointerMsg::Up(p) => p.position,
        }
    }

    pub fn button(&self) -> i16 {
        match self {
            PointerMsg::Down(p) | PointerMsg::Move(p) | PointerMsg::Up(p) => p.button,
        }
    }

    pub fn id(&self) -> i32 {
        match self {
            PointerMsg::Down(p) | PointerMsg::Move(p) | PointerMsg::Up(p) => p.id,
        }
    }
}

#[derive(Debug)]
/// Details of a pointer event.
pub struct PointerDetails {
    pub id: i32,
    pub button: i16,
    pub position: Point,
}

impl PointerDetails {
    fn from_pointer_event(e: &PointerEvent) -> Self {
        PointerDetails {
            id: e.pointer_id(),
            button: e.button(),
            position: Point::new(e.client_x() as f64, e.client_y() as f64),
        }
    }
}

pub fn pointer<T, A, F: Fn(&mut T, PointerMsg), V: Element<T, A>>(
    child: V,
    callback: F,
) -> Pointer<V, T, A, F> {
    Pointer {
        child,
        callback,
        phantom: Default::default(),
    }
}

fn build_event_listeners(
    ctx: &mut ViewCtx,
    el: &web_sys::Element,
) -> [Closure<dyn FnMut(PointerEvent)>; 3] {
    let el_clone = el.clone();

    let thunk = ctx.message_thunk();
    let down_closure = Closure::new(move |e: PointerEvent| {
        thunk.push_message(PointerMsg::Down(PointerDetails::from_pointer_event(&e)));
        el_clone.set_pointer_capture(e.pointer_id()).unwrap();
        e.prevent_default();
        e.stop_propagation();
    });
    el.add_event_listener_with_callback("pointerdown", down_closure.as_ref().unchecked_ref())
        .unwrap();

    let thunk = ctx.message_thunk();
    let move_closure = Closure::new(move |e: PointerEvent| {
        thunk.push_message(PointerMsg::Move(PointerDetails::from_pointer_event(&e)));
        e.prevent_default();
        e.stop_propagation();
    });
    el.add_event_listener_with_callback("pointermove", move_closure.as_ref().unchecked_ref())
        .unwrap();

    let thunk = ctx.message_thunk();
    let up_closure = Closure::new(move |e: PointerEvent| {
        thunk.push_message(PointerMsg::Up(PointerDetails::from_pointer_event(&e)));
        e.prevent_default();
        e.stop_propagation();
    });
    el.add_event_listener_with_callback("pointerup", up_closure.as_ref().unchecked_ref())
        .unwrap();

    [down_closure, move_closure, up_closure]
}

impl<V, State, Action, Callback> ViewMarker for Pointer<V, State, Action, Callback> {}
impl<State, Action, Callback, V> View<State, Action, ViewCtx, DynMessage>
    for Pointer<V, State, Action, Callback>
where
    State: 'static,
    Action: 'static,
    Callback: Fn(&mut State, PointerMsg) -> Action + 'static,
    V: DomView<State, Action, DomNode: AsRef<web_sys::Element>>,
{
    type ViewState = PointerState<V::ViewState>;
    type Element = V::Element;

    fn build(&self, ctx: &mut ViewCtx) -> (Self::Element, Self::ViewState) {
        ctx.with_id(POINTER_VIEW_ID, |ctx| {
            let (element, child_state) = self.child.build(ctx);
            let el = element.node.as_ref();

            let [down_closure, move_closure, up_closure] = build_event_listeners(ctx, el);
            let state = PointerState {
                down_closure,
                move_closure,
                up_closure,
                child_state,
            };
            (element, state)
        })
    }

    fn rebuild(
        &self,
        prev: &Self,
        state: &mut Self::ViewState,
        ctx: &mut ViewCtx,
        mut el: Mut<Self::Element>,
    ) {
        ctx.with_id(POINTER_VIEW_ID, |ctx| {
            self.child
                .rebuild(&prev.child, &mut state.child_state, ctx, el.reborrow_mut());

            if el.flags.was_created() {
                [state.down_closure, state.move_closure, state.up_closure] =
                    build_event_listeners(ctx, el.node.as_ref());
            }
        });
    }

    fn teardown(
        &self,
        view_state: &mut Self::ViewState,
        ctx: &mut ViewCtx,
        element: Mut<Self::Element>,
    ) {
        ctx.with_id(POINTER_VIEW_ID, |ctx| {
            // TODO remove event listeners from child or is this not necessary?
            self.child
                .teardown(&mut view_state.child_state, ctx, element);
        });
    }

    fn message(
        &self,
        view_state: &mut Self::ViewState,
        id_path: &[ViewId],
        message: DynMessage,
        app_state: &mut State,
    ) -> MessageResult<Action, DynMessage> {
        let Some((first, remainder)) = id_path.split_first() else {
            throw_str("Parent view of `Pointer` sent outdated and/or incorrect empty view path");
        };
        if *first != POINTER_VIEW_ID {
            throw_str("Parent view of `Pointer` sent outdated and/or incorrect empty view path");
        }
        if remainder.is_empty() {
            let msg = message.downcast().unwrap_throw();
            MessageResult::Action((self.callback)(app_state, *msg))
        } else {
            self.child
                .message(&mut view_state.child_state, remainder, message, app_state)
        }
    }
}
