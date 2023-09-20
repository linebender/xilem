use std::borrow::Cow;

use wasm_bindgen::JsCast;

use crate::{attribute::Attr, event::OnEvent, OptionalAction, View, ViewMarker};

use super::Element;

// TODO should this have the super trait View or should Node be the one?
// And/Or should the View::Element use EventTarget instead of Node (currently the trait `DomNode`)?
pub trait EventTarget<T, A>: View<T, A> + ViewMarker {
    fn on<E, EH, OA>(self, event: impl Into<Cow<'static, str>>, handler: EH) -> OnEvent<Self, E, EH>
    where
        E: JsCast + 'static,
        OA: OptionalAction<A>,
        EH: Fn(&mut T, E) -> OA,
        Self: Sized,
    {
        OnEvent::new(self, event, handler)
    }
}

impl<T, A, E: Element<T, A>> EventTarget<T, A> for Attr<E> {}
impl<T, A, E: EventTarget<T, A>, Ev, F, OA> EventTarget<T, A> for OnEvent<E, Ev, F>
where
    F: Fn(&mut T, Ev) -> OA,
    E: EventTarget<T, A>,
    Ev: JsCast + 'static,
    OA: OptionalAction<A>,
{
}
