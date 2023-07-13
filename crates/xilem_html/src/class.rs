// Copyright 2023 the Druid Authors.
// SPDX-License-Identifier: Apache-2.0

use std::{any::Any, borrow::Cow};

use wasm_bindgen::UnwrapThrowExt;
use xilem_core::{Id, MessageResult};

use crate::{
    context::{ChangeFlags, Cx},
    element::ElementState,
    view::{DomElement, View, ViewMarker},
};

pub struct Class<V> {
    child: V,
    class: Cow<'static, str>,
}

/// Add a class to the child element. Adding the empty class is equivalent to not adding a class.
pub fn class<V>(child: V, class: impl Into<Cow<'static, str>>) -> Class<V> {
    Class {
        child,
        class: class.into(),
    }
}

impl<V> ViewMarker for Class<V> {}

// TODO: make generic over A (probably requires Phantom)
impl<T, A, V, CS> View<T, A> for Class<V>
where
    V: View<T, A, State = ElementState<CS>>,
    V::Element: DomElement,
{
    type State = ElementState<CS>;
    type Element = V::Element;

    fn build(&self, cx: &mut Cx) -> (Id, Self::State, Self::Element) {
        let (id, mut child_state, element) = self.child.build(cx);
        if self.class != "" {
            element
                .as_element_ref()
                .class_list()
                .add_1(&self.class)
                .unwrap_throw();
        }
        child_state.init_class(self.class.to_string());
        (id, child_state, element)
    }

    fn rebuild(
        &self,
        cx: &mut Cx,
        prev: &Self,
        id: &mut Id,
        state: &mut Self::State,
        element: &mut V::Element,
    ) -> ChangeFlags {
        // TODO what if ID changed?
        state.add_class(self.class.to_string());
        self.child.rebuild(cx, &prev.child, id, state, element)
    }

    fn message(
        &self,
        id_path: &[Id],
        state: &mut Self::State,
        message: Box<dyn Any>,
        app_state: &mut T,
    ) -> MessageResult<A> {
        self.child.message(id_path, state, message, app_state)
    }
}

/// Helper to add a class only when `should_apply` is true.
pub fn opt_class<V>(child: V, class: impl Into<Cow<'static, str>>, should_apply: bool) -> Class<V> {
    Class {
        child,
        class: if should_apply {
            class.into()
        } else {
            Cow::Borrowed("")
        },
    }
}
