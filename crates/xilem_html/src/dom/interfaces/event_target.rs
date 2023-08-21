use std::{borrow::Cow, marker::PhantomData};

use gloo::events::EventListenerOptions;

use crate::dom::{attribute::Attr, event::EventListener};

pub trait EventTarget {
    fn on<T, EH, E, OA>(
        self,
        event: impl Into<Cow<'static, str>>,
        handler: EH,
    ) -> EventListener<Self, E, EH>
    where
        EH: Fn(&mut T, E) -> OA,
        Self: Sized,
    {
        EventListener {
            event: event.into(),
            element: self,
            event_handler_options: Default::default(),
            handler,
            phantom_event_ty: PhantomData,
        }
    }
    fn on_with_options<T, EH, E, OA>(
        self,
        event: impl Into<Cow<'static, str>>,
        handler: EH,
        options: EventListenerOptions,
    ) -> EventListener<Self, E, EH>
    where
        EH: Fn(&mut T, E) -> OA,
        Self: Sized,
    {
        EventListener {
            event: event.into(),
            element: self,
            event_handler_options: options,
            handler,
            phantom_event_ty: PhantomData,
        }
    }
}

impl<E: EventTarget> EventTarget for Attr<E> {}
impl<E: EventTarget, Ev, F> EventTarget for EventListener<E, Ev, F> {}
