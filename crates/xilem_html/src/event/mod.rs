// Copyright 2023 the Druid Authors.
// SPDX-License-Identifier: Apache-2.0

#[cfg(feature = "typed")]
pub mod events;

use std::{any::Any, marker::PhantomData, ops::Deref};

use gloo::events::{EventListener, EventListenerOptions};
use wasm_bindgen::{JsCast, UnwrapThrowExt};
use xilem_core::{Id, MessageResult};

use crate::{
    context::{ChangeFlags, Cx},
    view::{DomNode, View, ViewMarker},
};

/// Wraps a [`View`] `V` and attaches an event listener.
///
/// The event type `E` contains both the [`web_sys::Event`] subclass for this event and the
/// [`web_sys::HtmlElement`] subclass that matches `V::Element`.
pub struct OnEvent<E, V, F> {
    // TODO changing this after creation is unsupported for now,
    // please create a new view instead.
    event: &'static str,
    child: V,
    passive: bool,
    callback: F,
    phantom_event_ty: PhantomData<E>,
}

impl<E, V, F> OnEvent<E, V, F> {
    fn new(event: &'static str, child: V, callback: F) -> Self {
        Self {
            event,
            child,
            callback,
            passive: true,
            phantom_event_ty: PhantomData,
        }
    }

    /// Whether the event handler should be passive. (default = `true`)
    ///
    /// Passive event handlers can't prevent the browser's default action from
    /// running (otherwise possible with `event.prevent_default()`), which
    /// restricts what they can be used for, but reduces overhead.
    pub fn passive(mut self, value: bool) -> Self {
        self.passive = value;
        self
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
        let listener = EventListener::new_with_options(
            element.as_node_ref(),
            self.event,
            EventListenerOptions {
                passive: self.passive,
                ..Default::default()
            },
            move |event: &web_sys::Event| {
                let event = (*event).clone().dyn_into::<E>().unwrap_throw();
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
        match message.downcast_ref::<EventMsg<Event<E, V::Element>>>() {
            Some(msg) if id_path.is_empty() => {
                match (self.callback)(app_state, &msg.event).action() {
                    Some(a) => MessageResult::Action(a),
                    None => MessageResult::Nop,
                }
            }
            _ => self
                .child
                .message(id_path, &mut state.child_state, message, app_state),
        }
    }
}

// Attach an event listener to the child's element
pub fn on_event<E, V, F>(name: &'static str, child: V, callback: F) -> OnEvent<E, V, F> {
    OnEvent::new(name, child, callback)
}

/// State for the `OnEvent` view.
pub struct OnEventState<S> {
    #[allow(unused)]
    listener: EventListener,
    child_state: S,
}

pub(crate) struct EventMsg<E> {
    pub(crate) event: E,
}

/// Wraps a `web_sys::Event` and provides auto downcasting for both the event and its target.
pub struct Event<Evt, El> {
    raw: Evt,
    el: PhantomData<El>,
}

impl<Evt, El> Event<Evt, El> {
    pub(crate) fn new(raw: Evt) -> Self {
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
    /// Get the event target element.
    ///
    /// Because this type knows its child view's element type, we can downcast to this type here.
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
/// (`Action`, `Option<Action>` or `()`). An implementation detail.
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
