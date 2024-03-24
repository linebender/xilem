use std::{borrow::Cow, marker::PhantomData};

use xilem_core::{Id, MessageResult};

use crate::{
    interfaces::{sealed::Sealed, Element},
    ChangeFlags, Cx, View, ViewMarker,
};

/// Applies a class to the underlying element.
pub struct Class<E, T, A> {
    pub(crate) element: E,
    pub(crate) class_name: Cow<'static, str>,
    pub(crate) phantom: PhantomData<fn() -> (T, A)>,
}

impl<E, T, A> ViewMarker for Class<E, T, A> {}
impl<E, T, A> Sealed for Class<E, T, A> {}

impl<E: Element<T, A>, T, A> View<T, A> for Class<E, T, A> {
    type State = E::State;
    type Element = E::Element;

    fn build(&self, cx: &mut Cx) -> (Id, Self::State, Self::Element) {
        cx.add_class_to_element(&self.class_name);
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
        cx.add_class_to_element(&self.class_name);
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

crate::interfaces::impl_dom_interfaces_for_ty!(Element, Class);
