// Copyright 2023 the Druid Authors.
// SPDX-License-Identifier: Apache-2.0

#[cfg(feature = "typed")]
pub mod events;

use std::{any::Any, marker::PhantomData, ops::Deref};

use wasm_bindgen::{prelude::Closure, JsCast, UnwrapThrowExt};
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

impl<T, A, E, F, V> View<T, A> for OnEvent<E, V, F>
where
    F: Fn(&mut T, &Event<E, V::Element>) -> MessageResult<A>,
    V: View<T, A>,
    E: JsCast + 'static,
    V::Element: 'static,
{
    type State = OnEventState<V::State>;

    type Element = V::Element;

    fn build(&self, cx: &mut Cx) -> (Id, Self::State, Self::Element) {
        let (id, child_state, element) = self.child.build(cx);
        let thunk = cx.with_id(id, |cx| cx.message_thunk());
        let closure = Closure::wrap(Box::new(move |event: web_sys::Event| {
            let event = event.dyn_into::<E>().unwrap_throw();
            let event: Event<E, V::Element> = Event::new(event);
            thunk.push_message(EventMsg { event });
        }) as Box<dyn FnMut(web_sys::Event)>);
        element
            .as_node_ref()
            .add_event_listener_with_callback(self.event, closure.as_ref().unchecked_ref())
            .unwrap_throw();
        // TODO add `remove_listener_with_callback` to clean up listener?
        let state = OnEventState {
            closure,
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
            (self.callback)(app_state, &msg.event)
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
    closure: Closure<dyn FnMut(web_sys::Event)>,
    child_state: S,
}
struct EventMsg<E> {
    event: E,
}

/*
// on input
pub fn on_input<T, A, F: Fn(&mut T, &web_sys::InputEvent) -> MessageResult<A>, V: View<T, A>>(
    child: V,
    callback: F,
) -> OnEvent<web_sys::InputEvent, V, F> {
    OnEvent::new("input", child, callback)
}

// on click
pub fn on_click<T, A, F: Fn(&mut T, &web_sys::Event) -> MessageResult<A>, V: View<T, A>>(
    child: V,
    callback: F,
) -> OnEvent<web_sys::Event, V, F> {
    OnEvent::new("click", child, callback)
}

// on click
pub fn on_dblclick<T, A, F: Fn(&mut T, &web_sys::Event) -> MessageResult<A>, V: View<T, A>>(
    child: V,
    callback: F,
) -> OnEvent<web_sys::Event, V, F> {
    OnEvent::new("dblclick", child, callback)
}

// on keydown
pub fn on_keydown<
    T,
    A,
    F: Fn(&mut T, &web_sys::KeyboardEvent) -> MessageResult<A>,
    V: View<T, A>,
>(
    child: V,
    callback: F,
) -> OnEvent<web_sys::KeyboardEvent, V, F> {
    OnEvent::new("keydown", child, callback)
}
*/

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

/*
/// Types that can be created from a `web_sys::Event`.
///
/// Implementations may make the assumption that the event
/// is a particular subtype (e.g. `InputEvent`) and panic
/// when this is not the case (although it's preferred to use
/// `throw_str` and friends).
pub trait FromEvent: 'static {
    /// Convert the given event into `self`, or panic.
    fn from_event(event: &web_sys::Event) -> Self;
}

#[derive(Debug)]
pub struct InputEvent {
    pub data: Option<String>,
    /// The value of `event.target.value`.
    pub value: String,
}

impl FromEvent for InputEvent {
    fn from_event(event: &web_sys::Event) -> Self {
        let event: &web_sys::InputEvent = event.dyn_ref().unwrap_throw();
        Self {
            data: event.data(),
            value: event
                .target()
                .unwrap_throw()
                .dyn_into::<web_sys::HtmlInputElement>()
                .unwrap_throw()
                .value(),
        }
    }
}

pub struct Event {}

impl FromEvent for Event {
    fn from_event(_event: &web_sys::Event) -> Self {
        Self {}
    }
}

pub struct KeyboardEvent {
    pub key: String,
}

impl FromEvent for KeyboardEvent {
    fn from_event(event: &web_sys::Event) -> Self {
        let event: &web_sys::KeyboardEvent = event.dyn_ref().unwrap();
        Self { key: event.key() }
    }
}
*/
