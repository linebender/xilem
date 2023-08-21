use std::borrow::Cow;

use xilem_core::{Id, MessageResult};

use crate::{
    view::DomElement, AttributeValue, ChangeFlags, Cx, IntoAttributeValue, View, ViewMarker,
};

use super::elements::ElementState;

pub struct Attr<E> {
    pub(crate) element: E,
    pub(crate) name: Cow<'static, str>,
    pub(crate) value: Option<AttributeValue>,
}

impl<E> Attr<E> {
    pub fn attr<K: Into<Cow<'static, str>>, V: IntoAttributeValue>(
        self,
        name: K,
        value: V,
    ) -> Attr<Self> {
        Attr {
            element: self,
            name: name.into(),
            value: value.into_attribute_value(),
        }
    }
}

impl<E> ViewMarker for Attr<E> {}

impl<T, A, E, ES> View<T, A> for Attr<E>
where
    E: View<T, A, State = ElementState<ES>>,
    E::Element: DomElement,
{
    type State = E::State;
    type Element = E::Element;

    fn build(&self, cx: &mut Cx) -> (Id, Self::State, Self::Element) {
        let (id, state, element) = self.element.build(cx);
        if let Some(value) = &self.value {
            let _ = element
                .as_element_ref()
                .set_attribute(&self.name, &value.serialize());
        }
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
        state.add_new_attribute(&self.name, &self.value);
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
