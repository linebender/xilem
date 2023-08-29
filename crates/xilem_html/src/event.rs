use std::{any::Any, borrow::Cow, marker::PhantomData};

pub use gloo::events::EventListenerOptions;
use wasm_bindgen::{JsCast, UnwrapThrowExt};
use xilem_core::{Id, MessageResult};

use crate::{view::DomNode, ChangeFlags, Cx, OptionalAction, View, ViewMarker};

/// Wraps a [`View`] `V` and attaches an event listener.
///
/// The event type `E` should inherit from [`web_sys::Event`]
pub struct EventListener<V, E, F> {
    pub(crate) element: V,
    pub(crate) event: Cow<'static, str>,
    pub(crate) options: EventListenerOptions,
    pub(crate) handler: F,
    pub(crate) phantom_event_ty: PhantomData<E>,
}

impl<V, E, F> EventListener<V, E, F>
where
    E: JsCast + 'static,
{
    pub fn new(element: V, event: impl Into<Cow<'static, str>>, handler: F) -> Self {
        EventListener {
            element,
            event: event.into(),
            options: Default::default(),
            handler,
            phantom_event_ty: PhantomData,
        }
    }

    pub fn new_with_options(
        element: V,
        event: impl Into<Cow<'static, str>>,
        handler: F,
        options: EventListenerOptions,
    ) -> Self {
        EventListener {
            element,
            event: event.into(),
            options,
            handler,
            phantom_event_ty: PhantomData,
        }
    }

    fn create_event_listener(
        &self,
        target: &web_sys::EventTarget,
        cx: &Cx,
    ) -> gloo::events::EventListener {
        let thunk = cx.message_thunk();
        gloo::events::EventListener::new_with_options(
            target,
            self.event.clone(),
            self.options,
            move |event: &web_sys::Event| {
                let event = (*event).clone().dyn_into::<E>().unwrap_throw();
                thunk.push_message(event);
            },
        )
    }
}

/// State for the `OnEvent` view.
pub struct EventListenerState<S> {
    #[allow(unused)]
    listener: gloo::events::EventListener,
    child_id: Id,
    child_state: S,
}

impl<V, E, F> ViewMarker for EventListener<V, E, F> {}

impl<T, A, E, F, V, OA> View<T, A> for EventListener<V, E, F>
where
    F: Fn(&mut T, E) -> OA,
    V: View<T, A>,
    E: JsCast + 'static,
    OA: OptionalAction<A>,
{
    type State = EventListenerState<V::State>;

    type Element = V::Element;

    fn build(&self, cx: &mut Cx) -> (Id, Self::State, Self::Element) {
        let (id, (element, state)) = cx.with_new_id(|cx| {
            // id is already stored in element state
            let (child_id, child_state, element) = self.element.build(cx);
            let listener = self.create_event_listener(element.as_node_ref(), cx);
            let state = EventListenerState {
                child_state,
                child_id,
                listener,
            };
            (element, state)
        });
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
            let mut changed =
                self.element
                    .rebuild(cx, &prev.element, id, &mut state.child_state, element);
            // TODO check equality of prev and current element somehow
            if prev.event != self.event || changed.contains(ChangeFlags::STRUCTURE) {
                state.listener = self.create_event_listener(element.as_node_ref(), cx);
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
            [element_id, rest_path @ ..] if *element_id == state.child_id => {
                self.element
                    .message(rest_path, &mut state.child_state, message, app_state)
            }
            _ => MessageResult::Stale(message),
        }
    }
}
