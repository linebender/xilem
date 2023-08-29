use std::borrow::Cow;

use wasm_bindgen::JsCast;
use xilem_core::{Id, MessageResult};

use crate::{view::DomNode, AttributeValue, ChangeFlags, Cx, View, ViewMarker};

use super::interfaces::Element;

pub struct Attr<E> {
    pub(crate) element: E,
    pub(crate) name: Cow<'static, str>,
    pub(crate) value: Option<AttributeValue>,
}

impl<E> ViewMarker for Attr<E> {}

impl<T, A, E: Element<T, A>> View<T, A> for Attr<E> {
    type State = E::State;
    type Element = E::Element;

    fn build(&self, cx: &mut Cx) -> (Id, Self::State, Self::Element) {
        let (id, state, element) = self.element.build(cx);
        if let Some(value) = &self.value {
            let _ = element
                .as_node_ref()
                .dyn_ref::<web_sys::Element>()
                .expect(
                    "The Attr view can only be used by Views,\
                     whose element inherits from `web_sys::Element`",
                )
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
