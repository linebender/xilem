// Copyright 2023 the Druid Authors.
// SPDX-License-Identifier: Apache-2.0

//! Interactivity with pointer events.

use std::{any::Any, marker::PhantomData};

use wasm_bindgen::{prelude::Closure, JsCast};
use web_sys::PointerEvent;

use xilem_core::{Id, MessageResult};

use crate::{
    context::{ChangeFlags, Cx},
    view::{DomElement, View, ViewMarker},
};

pub struct Pointer<T, V, F> {
    child: V,
    callback: F,
    phantom: PhantomData<T>,
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

pub fn pointer<T, F: Fn(&mut T, PointerMsg), V: View<T>>(
    child: V,
    callback: F,
) -> Pointer<T, V, F> {
    Pointer {
        child,
        callback,
        phantom: Default::default(),
    }
}

impl<T, V, F> ViewMarker for Pointer<T, V, F> {}

impl<T, F: Fn(&mut T, PointerMsg) + Send, V: View<T>> View<T> for Pointer<T, V, F> {
    type State = PointerState<V::State>;
    type Element = V::Element;

    fn build(&self, cx: &mut Cx) -> (Id, Self::State, Self::Element) {
        let (id, child_state, element) = self.child.build(cx);
        let thunk = cx.with_id(id, |cx| cx.message_thunk());
        let el_clone = element.as_element_ref().clone();
        let down_closure = Closure::new(move |e: PointerEvent| {
            thunk.push_message(PointerMsg::Down(PointerDetails::from_pointer_event(&e)));
            el_clone.set_pointer_capture(e.pointer_id()).unwrap();
            e.prevent_default();
            e.stop_propagation();
        });
        element
            .as_element_ref()
            .add_event_listener_with_callback("pointerdown", down_closure.as_ref().unchecked_ref())
            .unwrap();
        let thunk = cx.with_id(id, |cx| cx.message_thunk());
        let move_closure = Closure::new(move |e: PointerEvent| {
            thunk.push_message(PointerMsg::Move(PointerDetails::from_pointer_event(&e)));
            e.prevent_default();
            e.stop_propagation();
        });
        element
            .as_element_ref()
            .add_event_listener_with_callback("pointermove", move_closure.as_ref().unchecked_ref())
            .unwrap();
        let thunk = cx.with_id(id, |cx| cx.message_thunk());
        let up_closure = Closure::new(move |e: PointerEvent| {
            thunk.push_message(PointerMsg::Up(PointerDetails::from_pointer_event(&e)));
            e.prevent_default();
            e.stop_propagation();
        });
        element
            .as_element_ref()
            .add_event_listener_with_callback("pointerup", up_closure.as_ref().unchecked_ref())
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
    ) -> MessageResult<()> {
        match message.downcast() {
            Ok(msg) => {
                (self.callback)(app_state, *msg);
                MessageResult::Action(())
            }
            Err(message) => self
                .child
                .message(id_path, &mut state.child_state, message, app_state),
        }
    }
}
