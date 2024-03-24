use std::borrow::Cow;
use std::marker::PhantomData;

use xilem_core::{Id, MessageResult};

use crate::{interfaces::sealed::Sealed, ChangeFlags, Cx, View, ViewMarker};

use super::interfaces::Element;

pub struct Style<E, T, A> {
    pub(crate) element: E,
    pub(crate) name: Cow<'static, str>,
    pub(crate) value: Option<Cow<'static, str>>,
    pub(crate) phantom: PhantomData<fn() -> (T, A)>,
}

impl<E, T, A> ViewMarker for Style<E, T, A> {}
impl<E, T, A> Sealed for Style<E, T, A> {}

impl<E: Element<T, A>, T, A> View<T, A> for Style<E, T, A> {
    type State = E::State;
    type Element = E::Element;

    fn build(&self, cx: &mut Cx) -> (Id, Self::State, Self::Element) {
        if let Some(value) = &self.value {
            cx.add_style_to_element(&self.name, value);
        }
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
        if let Some(value) = &self.value {
            cx.add_style_to_element(&self.name, value);
        }
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

crate::interfaces::impl_dom_interfaces_for_ty!(Element, Style);
