// Copyright 2023 the Druid Authors.
// SPDX-License-Identifier: Apache-2.0

#[cfg(feature = "typed")]
pub mod events;

use std::{any::Any, marker::PhantomData, ops::Deref};

use gloo::events::EventListener;
use wasm_bindgen::{JsCast, UnwrapThrowExt};
use xilem_core::{Id, MessageResult};

use crate::{
    context::{ChangeFlags, Cx},
    view::{DomNode, View, ViewMarker},
};

pub struct OnEvent<E, V, F> {
    // TODO changing this after creation is unsupported for now,
    // please create a new view instead.
    event: &'static str,
    child: V,
    callback: F,
    phantom_event_ty: PhantomData<E>,
}

impl<E, V, F> OnEvent<E, V, F> {
    fn new(event: &'static str, child: V, callback: F) -> Self {
        Self {
            event,
            child,
            callback,
            phantom_event_ty: PhantomData,
        }
    }
}

impl<E, V, F> ViewMarker for OnEvent<E, V, F> {}

impl<T, A, E, F, V, OA> View<T, A> for OnEvent<E, V, F>
where
    F: Fn(&mut T, &Event<E, V::Element>) -> OA,
    V: View<T, A>,
    E: JsCast + 'static,
    V::Element: 'static,
    OA: OptionalAction<A>,
{
    type State = OnEventState<V::State>;

    type Element = V::Element;

    fn build(&self, cx: &mut Cx) -> (Id, Self::State, Self::Element) {
        let (id, child_state, element) = self.child.build(cx);
        let thunk = cx.with_id(id, |cx| cx.message_thunk());
        let listener = EventListener::new(
            element.as_node_ref(),
            self.event,
            move |event: &web_sys::Event| {
                let event = (*event).clone();
                let event = event.dyn_into::<E>().unwrap_throw();
                let event: Event<E, V::Element> = Event::new(event);
                thunk.push_message(EventMsg { event });
            },
        );
        // TODO add `remove_listener_with_callback` to clean up listener?
        let state = OnEventState {
            listener,
            child_state,
        };
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
        // TODO: if the child id changes (as can happen with AnyView), reinstall closure
        self.child
            .rebuild(cx, &prev.child, id, &mut state.child_state, element)
    }

    fn message(
        &self,
        id_path: &[Id],
        state: &mut Self::State,
        message: Box<dyn Any>,
        app_state: &mut T,
    ) -> MessageResult<A> {
        if let Some(msg) = message.downcast_ref::<EventMsg<Event<E, V::Element>>>() {
            match (self.callback)(app_state, &msg.event).action() {
                Some(a) => MessageResult::Action(a),
                None => MessageResult::Nop,
            }
        } else {
            self.child
                .message(id_path, &mut state.child_state, message, app_state)
        }
    }
}

// Attach an event listener to the child's element
pub fn on_event<E, V, F>(name: &'static str, child: V, callback: F) -> OnEvent<E, V, F> {
    OnEvent::new(name, child, callback)
}

pub struct OnEventState<S> {
    #[allow(unused)]
    listener: EventListener,
    child_state: S,
}
struct EventMsg<E> {
    event: E,
}

pub struct Event<Evt, El> {
    raw: Evt,
    el: PhantomData<El>,
}

impl<Evt, El> Event<Evt, El> {
    fn new(raw: Evt) -> Self {
        Self {
            raw,
            el: PhantomData,
        }
    }
}

impl<Evt, El> Event<Evt, El>
where
    Evt: AsRef<web_sys::Event>,
    El: JsCast,
{
    pub fn target(&self) -> El {
        let evt: &web_sys::Event = self.raw.as_ref();
        evt.target().unwrap_throw().dyn_into().unwrap_throw()
    }
}

impl<Evt, El> Deref for Event<Evt, El> {
    type Target = Evt;
    fn deref(&self) -> &Self::Target {
        &self.raw
    }
}

/// Implement this trait for types you want to use as actions.
///
/// The trait exists because otherwise we couldn't provide versions
/// of listeners that take `()`, `A` and `Option<A>`.
pub trait Action {}

/// Trait that allows callbacks to be polymorphic on return type
/// (`Action`, `Option<Action>` or `()`)
pub trait OptionalAction<A>: sealed::Sealed {
    fn action(self) -> Option<A>;
}
mod sealed {
    pub trait Sealed {}
}

impl sealed::Sealed for () {}
impl<A> OptionalAction<A> for () {
    fn action(self) -> Option<A> {
        None
    }
}

impl<A: Action> sealed::Sealed for A {}
impl<A: Action> OptionalAction<A> for A {
    fn action(self) -> Option<A> {
        Some(self)
    }
}

impl<A: Action> sealed::Sealed for Option<A> {}
impl<A: Action> OptionalAction<A> for Option<A> {
    fn action(self) -> Option<A> {
        self
    }
}
