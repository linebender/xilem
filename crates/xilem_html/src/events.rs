use crate::{
    interfaces::{for_all_dom_interfaces, sealed::Sealed, Element},
    view::DomNode,
    ChangeFlags, Cx, OptionalAction, View, ViewMarker,
};
use std::{any::Any, borrow::Cow, marker::PhantomData};
use wasm_bindgen::{JsCast, UnwrapThrowExt};
use xilem_core::{Id, MessageResult};

pub use gloo::events::EventListenerOptions;

/// Wraps a [`View`] `V` and attaches an event listener.
///
/// The event type `E` should inherit from [`web_sys::Event`]
pub struct OnEvent<V, E, F> {
    pub(crate) element: V,
    pub(crate) event: Cow<'static, str>,
    pub(crate) options: EventListenerOptions,
    pub(crate) handler: F,
    pub(crate) phantom_event_ty: PhantomData<E>,
}

impl<V, E, F> OnEvent<V, E, F>
where
    E: JsCast + 'static,
{
    pub fn new(element: V, event: impl Into<Cow<'static, str>>, handler: F) -> Self {
        OnEvent {
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
        OnEvent {
            element,
            event: event.into(),
            options,
            handler,
            phantom_event_ty: PhantomData,
        }
    }

    /// Whether the event handler should be passive. (default = `true`)
    ///
    /// Passive event handlers can't prevent the browser's default action from
    /// running (otherwise possible with `event.prevent_default()`), which
    /// restricts what they can be used for, but reduces overhead.
    pub fn passive(mut self, value: bool) -> Self {
        self.options.passive = value;
        self
    }
}

fn create_event_listener<E: JsCast + 'static>(
    target: &web_sys::EventTarget,
    event: impl Into<Cow<'static, str>>,
    options: EventListenerOptions,
    cx: &Cx,
) -> gloo::events::EventListener {
    let thunk = cx.message_thunk();
    gloo::events::EventListener::new_with_options(
        target,
        event,
        options,
        move |event: &web_sys::Event| {
            let event = (*event).clone().dyn_into::<E>().unwrap_throw();
            thunk.push_message(event);
        },
    )
}

/// State for the `OnEvent` view.
pub struct OnEventState<S> {
    #[allow(unused)]
    listener: gloo::events::EventListener,
    child_id: Id,
    child_state: S,
}

impl<V, E, F> ViewMarker for OnEvent<V, E, F> {}
impl<V, E, F> Sealed for OnEvent<V, E, F> {}

impl<T, A, E, F, V, OA> View<T, A> for OnEvent<V, E, F>
where
    OA: OptionalAction<A>,
    F: Fn(&mut T, E) -> OA,
    V: Element<T, A>,
    E: JsCast + 'static,
{
    type State = OnEventState<V::State>;

    type Element = V::Element;

    fn build(&self, cx: &mut Cx) -> (Id, Self::State, Self::Element) {
        let (id, (element, state)) = cx.with_new_id(|cx| {
            let (child_id, child_state, element) = self.element.build(cx);
            let listener = create_event_listener::<E>(
                element.as_node_ref(),
                self.event.clone(),
                self.options,
                cx,
            );
            let state = OnEventState {
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
            let prev_child_id = state.child_id;
            let mut changed = self.element.rebuild(
                cx,
                &prev.element,
                &mut state.child_id,
                &mut state.child_state,
                element,
            );
            if state.child_id != prev_child_id {
                changed |= ChangeFlags::OTHER_CHANGE;
            }
            // TODO check equality of prev and current element somehow
            if prev.event != self.event || changed.contains(ChangeFlags::STRUCTURE) {
                state.listener = create_event_listener::<E>(
                    element.as_node_ref(),
                    self.event.clone(),
                    self.options,
                    cx,
                );
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

macro_rules! impl_dom_interface_for_event_listener {
    ($dom_interface:ident) => {
        impl<T, A, E, Ev, F, OA> $crate::interfaces::$dom_interface<T, A> for OnEvent<E, Ev, F>
        where
            F: Fn(&mut T, Ev) -> OA,
            E: $crate::interfaces::$dom_interface<T, A>,
            Ev: JsCast + 'static,
            OA: OptionalAction<A>,
        {
        }
    };
}

for_all_dom_interfaces!(impl_dom_interface_for_event_listener);

macro_rules! impl_dom_interface_for_event {
    ($dom_interface:ident, $event_ty:ident, $web_sys_ty: ident) => {
        impl<T, A, E, C, OA> $crate::interfaces::$dom_interface<T, A>
            for $crate::events::$event_ty<T, A, E, C>
        where
            E: $crate::interfaces::$dom_interface<T, A>,
            OA: OptionalAction<A>,
            C: Fn(&mut T, web_sys::$web_sys_ty) -> OA,
        {
        }
    };
}

macro_rules! event_definitions {
    ($(($ty_name:ident, $event_name:literal, $web_sys_ty:ident)),*) => {
        $(
        for_all_dom_interfaces!(impl_dom_interface_for_event, $ty_name, $web_sys_ty);

        pub struct $ty_name<T, A, ET, C> {
            target: ET,
            callback: C,
            options: EventListenerOptions,
            phantom: PhantomData<fn() -> (T, A)>,
        }

        impl<T, A, ET, C> $ty_name<T, A, ET, C> {
            pub fn new(target: ET, callback: C) -> Self {
                Self {
                    target,
                    options: Default::default(),
                    callback,
                    phantom: PhantomData,
                }
            }

            /// Whether the event handler should be passive. (default = `true`)
            ///
            /// Passive event handlers can't prevent the browser's default action from
            /// running (otherwise possible with `event.prevent_default()`), which
            /// restricts what they can be used for, but reduces overhead.
            pub fn passive(mut self, value: bool) -> Self {
                self.options.passive = value;
                self
            }
        }

        impl<T, A, ET, C> ViewMarker for $ty_name<T, A, ET, C> {}
        impl<T, A, ET, C> Sealed for $ty_name<T, A, ET, C> {}

        impl<T, A, C, ET, OA> View<T, A> for $ty_name<T, A, ET, C>
        where
            OA: OptionalAction<A>,
            C: Fn(&mut T, web_sys::$web_sys_ty) -> OA,
            ET: Element<T, A>,
        {
            type State = OnEventState<ET::State>;

            type Element = ET::Element;

            fn build(&self, cx: &mut Cx) -> (Id, Self::State, Self::Element) {
                let (id, (element, state)) = cx.with_new_id(|cx| {
                    let (child_id, child_state, el) = self.target.build(cx);
                    let listener = create_event_listener::<web_sys::$web_sys_ty>(el.as_node_ref(), $event_name, self.options, cx);
                    (el, OnEventState { child_state, child_id, listener })
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
                    let prev_child_id = state.child_id;
                    let mut changed = self.target.rebuild(cx, &prev.target, &mut state.child_id, &mut state.child_state, element);
                    if state.child_id != prev_child_id {
                        changed |= ChangeFlags::OTHER_CHANGE;
                    }
                    // TODO check equality of prev and current element somehow
                    if changed.contains(ChangeFlags::STRUCTURE) {
                        state.listener = create_event_listener::<web_sys::$web_sys_ty>(element.as_node_ref(), $event_name, self.options, cx);
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
                    [] if message.downcast_ref::<web_sys::$web_sys_ty>().is_some() => {
                        let event = message.downcast::<web_sys::$web_sys_ty>().unwrap();
                        match (self.callback)(app_state, *event).action() {
                            Some(a) => MessageResult::Action(a),
                            None => MessageResult::Nop,
                        }
                    }
                    [element_id, rest_path @ ..] if *element_id == state.child_id => {
                        self.target.message(rest_path, &mut state.child_state, message, app_state)
                    }
                    _ => MessageResult::Stale(message),
                }
            }
        }
        )*
    };
}

// click/auxclick/contextmenu are still mouse events in either Safari as well as Firefox,
// see: https://stackoverflow.com/questions/70626381/why-chrome-emits-pointerevents-and-firefox-mouseevents-and-which-type-definition/76900433#76900433
event_definitions!(
    (OnAbort, "abort", Event),
    (OnAuxClick, "auxclick", MouseEvent),
    (OnBeforeInput, "beforeinput", InputEvent),
    (OnBeforeMatch, "beforematch", Event),
    (OnBeforeToggle, "beforetoggle", Event),
    (OnBlur, "blur", FocusEvent),
    (OnCancel, "cancel", Event),
    (OnCanPlay, "canplay", Event),
    (OnCanPlayThrough, "canplaythrough", Event),
    (OnChange, "change", Event),
    (OnClick, "click", MouseEvent),
    (OnClose, "close", Event),
    (OnContextLost, "contextlost", Event),
    (OnContextMenu, "contextmenu", MouseEvent),
    (OnContextRestored, "contextrestored", Event),
    (OnCopy, "copy", Event),
    (OnCueChange, "cuechange", Event),
    (OnCut, "cut", Event),
    (OnDblClick, "dblclick", MouseEvent),
    (OnDrag, "drag", Event),
    (OnDragEnd, "dragend", Event),
    (OnDragEnter, "dragenter", Event),
    (OnDragLeave, "dragleave", Event),
    (OnDragOver, "dragover", Event),
    (OnDragStart, "dragstart", Event),
    (OnDrop, "drop", Event),
    (OnDurationChange, "durationchange", Event),
    (OnEmptied, "emptied", Event),
    (OnEnded, "ended", Event),
    (OnError, "error", Event),
    (OnFocus, "focus", FocusEvent),
    (OnFocusIn, "focusin", FocusEvent),
    (OnFocusOut, "focusout", FocusEvent),
    (OnFormData, "formdata", Event),
    (OnInput, "input", InputEvent),
    (OnInvalid, "invalid", Event),
    (OnKeyDown, "keydown", KeyboardEvent),
    (OnKeyUp, "keyup", KeyboardEvent),
    (OnLoad, "load", Event),
    (OnLoadedData, "loadeddata", Event),
    (OnLoadedMetadata, "loadedmetadata", Event),
    (OnLoadStart, "loadstart", Event),
    (OnMouseDown, "mousedown", MouseEvent),
    (OnMouseEnter, "mouseenter", MouseEvent),
    (OnMouseLeave, "mouseleave", MouseEvent),
    (OnMouseMove, "mousemove", MouseEvent),
    (OnMouseOut, "mouseout", MouseEvent),
    (OnMouseOver, "mouseover", MouseEvent),
    (OnMouseUp, "mouseup", MouseEvent),
    (OnPaste, "paste", Event),
    (OnPause, "pause", Event),
    (OnPlay, "play", Event),
    (OnPlaying, "playing", Event),
    (OnProgress, "progress", Event),
    (OnRateChange, "ratechange", Event),
    (OnReset, "reset", Event),
    (OnResize, "resize", Event),
    (OnScroll, "scroll", Event),
    (OnScrollEnd, "scrollend", Event),
    (OnSecurityPolicyViolation, "securitypolicyviolation", Event),
    (OnSeeked, "seeked", Event),
    (OnSeeking, "seeking", Event),
    (OnSelect, "select", Event),
    (OnSlotChange, "slotchange", Event),
    (OnStalled, "stalled", Event),
    (OnSubmit, "submit", Event),
    (OnSuspend, "suspend", Event),
    (OnTimeUpdate, "timeupdate", Event),
    (OnToggle, "toggle", Event),
    (OnVolumeChange, "volumechange", Event),
    (OnWaiting, "waiting", Event),
    (OnWheel, "wheel", WheelEvent)
);
