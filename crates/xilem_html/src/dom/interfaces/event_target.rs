use std::borrow::Cow;

use gloo::events::EventListenerOptions;
use wasm_bindgen::JsCast;

use crate::{
    dom::{attribute::Attr, event::EventListener},
    OptionalAction,
};

pub trait EventTarget {
    fn on<T, A, E, EH, OA>(
        self,
        event: impl Into<Cow<'static, str>>,
        handler: EH,
    ) -> EventListener<Self, E, EH>
    where
        E: JsCast + 'static,
        OA: OptionalAction<A>,
        EH: Fn(&mut T, E) -> OA,
        Self: Sized,
    {
        EventListener::new(self, event, handler)
    }

    fn on_with_options<T, A, E, EH, OA>(
        self,
        event: impl Into<Cow<'static, str>>,
        handler: EH,
        options: EventListenerOptions,
    ) -> EventListener<Self, E, EH>
    where
        E: JsCast + 'static,
        OA: OptionalAction<A>,
        EH: Fn(&mut T, E) -> OA,
        Self: Sized,
    {
        EventListener::new_with_options(self, event, handler, options)
    }
}

impl<E: EventTarget> EventTarget for Attr<E> {}
impl<E: EventTarget, Ev, F> EventTarget for EventListener<E, Ev, F> {}
