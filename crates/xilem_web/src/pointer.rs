// Copyright 2023 the Druid Authors.
// SPDX-License-Identifier: Apache-2.0

//! Interactivity with pointer events.

use std::{any::Any, marker::PhantomData};

use wasm_bindgen::{prelude::Closure, JsCast};
use web_sys::PointerEvent;

use xilem_core::{Id, MessageResult};

use crate::{
    context::{ChangeFlags, Cx},
    interfaces::Element,
    view::{DomNode, View, ViewMarker},
};

pub struct Pointer<V, T, A, F> {
    child: V,
    callback: F,
    phantom: PhantomData<fn() -> (T, A)>,
}

pub struct PointerState<S> {
    // Closures are retained so they can be called by environment
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

#[derive(Debug)]
/// Details of a pointer event.
pub struct PointerDetails {
    pub id: i32,
    pub button: i16,
    pub x: f64,
    pub y: f64,
}

impl PointerDetails {
    fn from_pointer_event(e: &PointerEvent) -> Self {
        PointerDetails {
            id: e.pointer_id(),
            button: e.button(),
            x: e.client_x() as f64,
            y: e.client_y() as f64,
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

crate::interfaces::impl_dom_interfaces_for_ty!(
    Element,
    Pointer,
    vars: <F,>,
    vars_on_ty: <F,>,
    bounds: {
        F: Fn(&mut T, PointerMsg) -> A,
    }
);

impl<V, T, A, F> ViewMarker for Pointer<V, T, A, F> {}
impl<V, T, A, F> crate::interfaces::sealed::Sealed for Pointer<V, T, A, F> {}

impl<T, A, F: Fn(&mut T, PointerMsg) -> A, V: View<T, A>> View<T, A> for Pointer<V, T, A, F> {
    type State = PointerState<V::State>;
    type Element = V::Element;

    fn build(&self, cx: &mut Cx) -> (Id, Self::State, Self::Element) {
        let (id, child_state, element) = self.child.build(cx);
        let thunk = cx.with_id(id, |cx| cx.message_thunk());
        let el = element.as_node_ref().dyn_ref::<web_sys::Element>().unwrap();
        let el_clone = el.clone();
        let down_closure = Closure::new(move |e: PointerEvent| {
            thunk.push_message(PointerMsg::Down(PointerDetails::from_pointer_event(&e)));
            el_clone.set_pointer_capture(e.pointer_id()).unwrap();
            e.prevent_default();
            e.stop_propagation();
        });
        el.add_event_listener_with_callback("pointerdown", down_closure.as_ref().unchecked_ref())
            .unwrap();
        let thunk = cx.with_id(id, |cx| cx.message_thunk());
        let move_closure = Closure::new(move |e: PointerEvent| {
            thunk.push_message(PointerMsg::Move(PointerDetails::from_pointer_event(&e)));
            e.prevent_default();
            e.stop_propagation();
        });
        el.add_event_listener_with_callback("pointermove", move_closure.as_ref().unchecked_ref())
            .unwrap();
        let thunk = cx.with_id(id, |cx| cx.message_thunk());
        let up_closure = Closure::new(move |e: PointerEvent| {
            thunk.push_message(PointerMsg::Up(PointerDetails::from_pointer_event(&e)));
            e.prevent_default();
            e.stop_propagation();
        });
        el.add_event_listener_with_callback("pointerup", up_closure.as_ref().unchecked_ref())
            .unwrap();
        let state = PointerState {
            down_closure,
            move_closure,
            up_closure,
            child_state,
        };
        (id, state, element)
    }

    fn rebuild(
        &self,
        cx: &mut Cx,
        prev: &Self,
        id: &mut Id,
        state: &mut Self::State,
        element: &mut Self::Element,
    ) -> ChangeFlags {
        // TODO: if the child id changes (as can happen with AnyView), reinstall closure
        self.child
            .rebuild(cx, &prev.child, id, &mut state.child_state, element)
    }

    fn message(
        &self,
        id_path: &[Id],
        state: &mut Self::State,
        message: Box<dyn Any>,
        app_state: &mut T,
    ) -> MessageResult<A> {
        match message.downcast() {
            Ok(msg) => MessageResult::Action((self.callback)(app_state, *msg)),
            Err(message) => self
                .child
                .message(id_path, &mut state.child_state, message, app_state),
        }
    }
}
