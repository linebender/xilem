// Copyright 2023 the Xilem Authors
// SPDX-License-Identifier: Apache-2.0

use std::borrow::Cow;
use std::marker::PhantomData;

use xilem_core::{Id, MessageResult};

use crate::{interfaces::sealed::Sealed, AttributeValue, ChangeFlags, Cx, View, ViewMarker};

use super::interfaces::Element;

pub struct Attr<E, T, A> {
    pub(crate) element: E,
    pub(crate) name: Cow<'static, str>,
    pub(crate) value: Option<AttributeValue>,
    pub(crate) phantom: PhantomData<fn() -> (T, A)>,
}

impl<E, T, A> ViewMarker for Attr<E, T, A> {}
impl<E, T, A> Sealed for Attr<E, T, A> {}

impl<E: Element<T, A>, T, A> View<T, A> for Attr<E, T, A> {
    type State = E::State;
    type Element = E::Element;

    fn build(&self, cx: &mut Cx) -> (Id, Self::State, Self::Element) {
        cx.add_attr_to_element(&self.name, &self.value);
        self.element.build(cx)
    }

    fn rebuild(
        &self,
        cx: &mut Cx,
        prev: &Self,
        id: &mut Id,
        state: &mut Self::State,
        element: &mut Self::Element,
    ) -> ChangeFlags {
        cx.add_attr_to_element(&self.name, &self.value);
        self.element.rebuild(cx, &prev.element, id, state, element)
    }

    fn message(
        &self,
        id_path: &[Id],
        state: &mut Self::State,
        message: Box<dyn std::any::Any>,
        app_state: &mut T,
    ) -> MessageResult<A> {
        self.element.message(id_path, state, message, app_state)
    }
}

crate::interfaces::impl_dom_interfaces_for_ty!(Element, Attr);
