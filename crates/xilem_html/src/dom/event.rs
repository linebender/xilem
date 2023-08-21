use std::{any::Any, borrow::Cow, marker::PhantomData};

use gloo::events::EventListenerOptions;
use wasm_bindgen::{JsCast, UnwrapThrowExt};
use xilem_core::{Id, MessageResult};

use crate::{view::DomNode, ChangeFlags, Cx, OptionalAction, View, ViewMarker};

use super::elements::ElementState;

/// Wraps a [`View`] `V` and attaches an event listener.
///
/// The event type `E` contains both the [`web_sys::Event`] subclass for this event and the
/// [`web_sys::HtmlElement`] subclass that matches `V::Element`.
pub struct EventListener<V, E, F> {
    pub(crate) element: V,
    pub(crate) event: Cow<'static, str>,
    pub(crate) event_handler_options: EventListenerOptions,
    pub(crate) handler: F,
    pub(crate) phantom_event_ty: PhantomData<E>,
}

impl<V, E, F> EventListener<V, E, F>
where
    E: JsCast + 'static,
{
    fn create_event_listener(
        &self,
        target: &web_sys::EventTarget,
        cx: &Cx,
    ) -> gloo::events::EventListener {
        let thunk = cx.message_thunk();
        gloo::events::EventListener::new_with_options(
            target,
            self.event.clone(),
            self.event_handler_options,
            move |event: &web_sys::Event| {
                let event = (*event).clone().dyn_into::<E>().unwrap_throw();
                thunk.push_message(event);
            },
        )
    }
}

impl<V, E, F> ViewMarker for EventListener<V, E, F> {}

impl<T, A, E, F, V, ES, OA> View<T, A> for EventListener<V, E, F>
where
    F: Fn(&mut T, E) -> OA,
    V: View<T, A, State = ElementState<ES>>,
    E: JsCast + 'static,
    OA: OptionalAction<A>,
{
    type State = V::State;

    type Element = V::Element;

    fn build(&self, cx: &mut Cx) -> (Id, Self::State, Self::Element) {
        let (id, (mut state, element, listener)) = cx.with_new_id(|cx| {
            // id is already stored in element state
            let (_id, state, element) = self.element.build(cx);
            let listener = self.create_event_listener(element.as_node_ref(), cx);
            (state, element, listener)
        });
        state.add_new_listener(id, listener);
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
        cx.with_id(*id, |cx| {
            let mut changed = self.element.rebuild(cx, &prev.element, id, state, element);
            // TODO check equality of prev and current element
            if prev.event != self.event || changed.contains(ChangeFlags::STRUCTURE) {
                let new_listener = self.create_event_listener(element.as_node_ref(), cx);
                if let Some(listener) = state.get_listener(*id) {
                    *listener = new_listener;
                } else {
                    state.add_new_listener(*id, new_listener);
                }
                changed |= ChangeFlags::OTHER_CHANGE;
            }
            changed
        })
    }

    fn message(
        &self,
        id_path: &[Id],
        state: &mut Self::State,
        message: Box<dyn Any>,
        app_state: &mut T,
    ) -> MessageResult<A> {
        match id_path {
            [] if message.downcast_ref::<E>().is_some() => {
                let event = message.downcast::<E>().unwrap();
                match (self.handler)(app_state, *event).action() {
                    Some(a) => MessageResult::Action(a),
                    None => MessageResult::Nop,
                }
            }
            [element_id, rest_path @ ..] if *element_id == state.id => {
                self.element.message(rest_path, state, message, app_state)
            }
            _ => MessageResult::Stale(message),
        }
    }
}
