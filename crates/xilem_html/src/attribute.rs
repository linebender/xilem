use std::borrow::Cow;
use std::marker::PhantomData;

use xilem_core::{Id, MessageResult};

use crate::{interfaces::sealed::Sealed, AttributeValue, ChangeFlags, Cx, View, ViewMarker};

use super::interfaces::{for_all_dom_interfaces, Element};

pub struct Attr<T, A, E> {
    pub(crate) element: E,
    pub(crate) name: Cow<'static, str>,
    pub(crate) value: Option<AttributeValue>,
    pub(crate) phantom: PhantomData<fn() -> (T, A)>,
}

impl<T, A, E> ViewMarker for Attr<T, A, E> {}
impl<T, A, E> Sealed for Attr<T, A, E> {}

impl<T, A, E: Element<T, A>> View<T, A> for Attr<T, A, E> {
    type State = E::State;
    type Element = E::Element;

    fn build(&self, cx: &mut Cx) -> (Id, Self::State, Self::Element) {
        cx.add_new_attribute_to_current_element(&self.name, &self.value);
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
        cx.add_new_attribute_to_current_element(&self.name, &self.value);
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

macro_rules! impl_dom_interface_for_attr {
    ($dom_interface:ident) => {
        impl<T, A, E: $crate::interfaces::$dom_interface<T, A>>
            $crate::interfaces::$dom_interface<T, A> for Attr<T, A, E>
        {
        }
    };
}

for_all_dom_interfaces!(impl_dom_interface_for_attr);
