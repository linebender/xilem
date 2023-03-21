use std::borrow::Cow;
use wasm_bindgen::JsCast;

use xilem_core::{Id, MessageResult};

use crate::{
    context::{ChangeFlags, Cx},
    view::{View, ViewMarker},
};

pub struct Text {
    text: Cow<'static, str>,
}

/// Create a text node
pub fn text(text: impl Into<Cow<'static, str>>) -> Text {
    Text { text: text.into() }
}

impl ViewMarker for Text {}

impl<T, A> View<T, A> for Text {
    type State = ();
    type Element = web_sys::Text;

    fn build(&self, _cx: &mut Cx) -> (Id, Self::State, Self::Element) {
        let el = new_text(&self.text);
        let id = Id::next();
        (id, (), el.unchecked_into())
    }

    fn rebuild(
        &self,
        _cx: &mut Cx,
        prev: &Self,
        _id: &mut Id,
        _state: &mut Self::State,
        element: &mut Self::Element,
    ) -> ChangeFlags {
        let mut is_changed = ChangeFlags::empty();
        if prev.text != self.text {
            element.set_data(&self.text);
            is_changed |= ChangeFlags::OTHER_CHANGE;
        }
        is_changed
    }

    fn message(
        &self,
        _id_path: &[Id],
        _state: &mut Self::State,
        _message: Box<dyn std::any::Any>,
        _app_state: &mut T,
    ) -> MessageResult<A> {
        MessageResult::Nop
    }
}

fn new_text(text: &str) -> web_sys::Text {
    web_sys::Text::new_with_data(text).unwrap()
}
