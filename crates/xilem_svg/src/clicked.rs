// Copyright 2023 the Druid Authors.
// SPDX-License-Identifier: Apache-2.0

use std::any::Any;

use wasm_bindgen::{prelude::Closure, JsCast};
use web_sys::SvgElement;

use xilem_core::{Id, MessageResult};

use crate::{
    context::{ChangeFlags, Cx},
    view::{DomElement, View, ViewMarker},
};

pub struct Clicked<V, F> {
    child: V,
    callback: F,
}

pub struct ClickedState<S> {
    // Closure is retained so it can be called by environment
    #[allow(unused)]
    closure: Closure<dyn FnMut()>,
    child_state: S,
}

struct ClickedMsg;

pub fn clicked<T, F: Fn(&mut T), V: View<T>>(child: V, callback: F) -> Clicked<V, F> {
    Clicked { child, callback }
}

impl<V, F> ViewMarker for Clicked<V, F> {}

impl<T, F: Fn(&mut T) + Send, V: View<T>> View<T> for Clicked<V, F> {
    type State = ClickedState<V::State>;

    type Element = V::Element;

    fn build(&self, cx: &mut Cx) -> (Id, Self::State, Self::Element) {
        let (id, child_state, element) = self.child.build(cx);
        let thunk = cx.with_id(id, |cx| cx.message_thunk());
        let closure =
            Closure::wrap(Box::new(move || thunk.push_message(ClickedMsg)) as Box<dyn FnMut()>);
        element
            .as_element_ref()
            .dyn_ref::<SvgElement>()
            .expect("not an svg element")
            .set_onclick(Some(closure.as_ref().unchecked_ref()));
        let state = ClickedState {
            closure,
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
        if message.downcast_ref::<ClickedMsg>().is_some() {
            (self.callback)(app_state);
            MessageResult::Action(())
        } else {
            self.child
                .message(id_path, &mut state.child_state, message, app_state)
        }
    }
}
