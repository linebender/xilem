// Copyright 2023 the Xilem Authors
// SPDX-License-Identifier: Apache-2.0

use std::{borrow::Cow, marker::PhantomData};
use wasm_bindgen::{prelude::Closure, throw_str, JsCast, UnwrapThrowExt};
use web_sys::{js_sys, AddEventListenerOptions};
use xilem_core::{MessageResult, Mut, View, ViewId, ViewMarker, ViewPathTracker};

use crate::{DynMessage, ElementAsRef, OptionalAction, ViewCtx};

/// Use a distinctive number here, to be able to catch bugs.
/// In case the generational-id view path in `View::Message` lead to a wrong view
const ON_EVENT_VIEW_ID: ViewId = ViewId::new(0x2357_1113);

/// Wraps a [`View`] `V` and attaches an event listener.
///
/// The event type `Event` should inherit from [`web_sys::Event`]
#[derive(Clone, Debug)]
pub struct OnEvent<V, State, Action, Event, Callback> {
    pub(crate) element: V,
    pub(crate) event: Cow<'static, str>,
    pub(crate) capture: bool,
    pub(crate) passive: bool,
    pub(crate) handler: Callback,
    pub(crate) phantom_event_ty: PhantomData<fn() -> (State, Action, Event)>,
}

impl<V, State, Action, Event, Callback> OnEvent<V, State, Action, Event, Callback>
where
    Event: JsCast + 'static,
{
    pub fn new(element: V, event: impl Into<Cow<'static, str>>, handler: Callback) -> Self {
        OnEvent {
            element,
            event: event.into(),
            passive: true,
            capture: false,
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
        self.passive = value;
        self
    }

    /// Whether the event handler should capture the event *before* being dispatched to any `EventTarget` beneath it in the DOM tree. (default = `false`)
    ///
    /// Events that are bubbling upward through the tree will not trigger a listener designated to use capture.
    /// Event bubbling and capturing are two ways of propagating events that occur in an element that is nested within another element,
    /// when both elements have registered a handle for that event.
    /// The event propagation mode determines the order in which elements receive the event.
    // TODO use similar Nomenclature as gloo (Phase::Bubble/Phase::Capture)?
    pub fn capture(mut self, value: bool) -> Self {
        self.capture = value;
        self
    }
}

fn create_event_listener<Event: JsCast + crate::Message>(
    target: &web_sys::EventTarget,
    event: &str,
    // TODO options
    capture: bool,
    passive: bool,
    ctx: &mut ViewCtx,
) -> Closure<dyn FnMut(web_sys::Event)> {
    let thunk = ctx.message_thunk();
    let callback = Closure::new(move |event: web_sys::Event| {
        let event = event.unchecked_into::<Event>();
        thunk.push_message(event);
    });

    let mut options = AddEventListenerOptions::new();
    options.capture(capture);
    options.passive(passive);

    target
        .add_event_listener_with_callback_and_add_event_listener_options(
            event,
            callback.as_ref().unchecked_ref(),
            &options,
        )
        .unwrap_throw();
    callback
}

fn remove_event_listener(
    target: &web_sys::EventTarget,
    event: &str,
    callback: &Closure<dyn FnMut(web_sys::Event)>,
    is_capture: bool,
) {
    target
        .remove_event_listener_with_callback_and_bool(
            event,
            callback.as_ref().unchecked_ref(),
            is_capture,
        )
        .unwrap_throw();
}

/// State for the `OnEvent` view.
pub struct OnEventState<S> {
    #[allow(unused)]
    child_state: S,
    callback: Closure<dyn FnMut(web_sys::Event)>,
}

// These (boilerplatey) functions are there to reduce the boilerplate created by the macro-expansion below.

fn build_event_listener<State, Action, V, Event>(
    element_view: &V,
    event: &str,
    capture: bool,
    passive: bool,
    ctx: &mut ViewCtx,
) -> (V::Element, OnEventState<V::ViewState>)
where
    State: 'static,
    Action: 'static,
    V: View<State, Action, ViewCtx, DynMessage>,
    V::Element: ElementAsRef<web_sys::EventTarget>,
    Event: JsCast + 'static + crate::Message,
{
    // we use a placeholder id here, the id can never change, so we don't need to store it anywhere
    ctx.with_id(ON_EVENT_VIEW_ID, |ctx| {
        let (element, child_state) = element_view.build(ctx);
        let callback =
            create_event_listener::<Event>(element.as_ref(), event, capture, passive, ctx);
        let state = OnEventState {
            child_state,
            callback,
        };
        (element, state)
    })
}

#[allow(clippy::too_many_arguments)]
fn rebuild_event_listener<'el, State, Action, V, Event>(
    element_view: &V,
    prev_element_view: &V,
    element: Mut<'el, V::Element>,
    event: &str,
    capture: bool,
    passive: bool,
    prev_capture: bool,
    prev_passive: bool,
    state: &mut OnEventState<V::ViewState>,
    ctx: &mut ViewCtx,
) -> Mut<'el, V::Element>
where
    State: 'static,
    Action: 'static,
    V: View<State, Action, ViewCtx, DynMessage>,
    V::Element: ElementAsRef<web_sys::EventTarget>,
    Event: JsCast + 'static + crate::Message,
{
    ctx.with_id(ON_EVENT_VIEW_ID, |ctx| {
        if prev_capture != capture || prev_passive != passive {
            remove_event_listener(element.as_ref(), event, &state.callback, prev_capture);

            state.callback =
                create_event_listener::<Event>(element.as_ref(), event, capture, passive, ctx);
        }
        element_view.rebuild(prev_element_view, &mut state.child_state, ctx, element)
    })
}

fn teardown_event_listener<State, Action, V>(
    element_view: &V,
    element: Mut<'_, V::Element>,
    _event: &str,
    state: &mut OnEventState<V::ViewState>,
    _capture: bool,
    ctx: &mut ViewCtx,
) where
    State: 'static,
    Action: 'static,
    V: View<State, Action, ViewCtx, DynMessage>,
    V::Element: ElementAsRef<web_sys::EventTarget>,
{
    // TODO: is this really needed (as the element will be removed anyway)?
    // remove_event_listener(element.as_ref(), event, &state.callback, capture);
    ctx.with_id(ON_EVENT_VIEW_ID, |ctx| {
        element_view.teardown(&mut state.child_state, ctx, element);
    });
}

fn message_event_listener<State, Action, V, Event, OA, Callback>(
    element_view: &V,
    state: &mut OnEventState<V::ViewState>,
    id_path: &[ViewId],
    message: DynMessage,
    app_state: &mut State,
    handler: &Callback,
) -> MessageResult<Action, DynMessage>
where
    State: 'static,
    Action: 'static,
    V: View<State, Action, ViewCtx, DynMessage>,
    V::Element: ElementAsRef<web_sys::EventTarget>,
    Event: JsCast + 'static + crate::Message,
    OA: OptionalAction<Action>,
    Callback: Fn(&mut State, Event) -> OA + 'static,
{
    let Some((first, remainder)) = id_path.split_first() else {
        throw_str("Parent view of `OnEvent` sent outdated and/or incorrect empty view path");
    };
    if *first != ON_EVENT_VIEW_ID {
        throw_str("Parent view of `OnEvent` sent outdated and/or incorrect empty view path");
    }
    if remainder.is_empty() {
        let event = message.downcast::<Event>().unwrap_throw();
        match (handler)(app_state, *event).action() {
            Some(a) => MessageResult::Action(a),
            None => MessageResult::Nop,
        }
    } else {
        element_view.message(&mut state.child_state, remainder, message, app_state)
    }
}

impl<V, State, Action, Event, Callback> ViewMarker for OnEvent<V, State, Action, Event, Callback> {}
impl<V, State, Action, Event, Callback, OA> View<State, Action, ViewCtx, DynMessage>
    for OnEvent<V, State, Action, Event, Callback>
where
    State: 'static,
    Action: 'static,
    OA: OptionalAction<Action>,
    Callback: Fn(&mut State, Event) -> OA + 'static,
    V: View<State, Action, ViewCtx, DynMessage>,
    V::Element: ElementAsRef<web_sys::EventTarget>,
    Event: JsCast + 'static + crate::Message,
{
    type ViewState = OnEventState<V::ViewState>;

    type Element = V::Element;

    fn build(&self, ctx: &mut ViewCtx) -> (Self::Element, Self::ViewState) {
        build_event_listener::<_, _, _, Event>(
            &self.element,
            &self.event,
            self.capture,
            self.passive,
            ctx,
        )
    }

    fn rebuild<'el>(
        &self,
        prev: &Self,
        view_state: &mut Self::ViewState,
        ctx: &mut ViewCtx,
        element: Mut<'el, Self::Element>,
    ) -> Mut<'el, Self::Element> {
        // special case, where event name can change, so we can't reuse the rebuild_event_listener function above
        ctx.with_id(ON_EVENT_VIEW_ID, |ctx| {
            if prev.capture != self.capture
                || prev.passive != self.passive
                || prev.event != self.event
            {
                remove_event_listener(
                    element.as_ref(),
                    &prev.event,
                    &view_state.callback,
                    prev.capture,
                );

                view_state.callback = create_event_listener::<Event>(
                    element.as_ref(),
                    &self.event,
                    self.capture,
                    self.passive,
                    ctx,
                );
            }
            self.element
                .rebuild(&prev.element, &mut view_state.child_state, ctx, element)
        })
    }

    fn teardown(
        &self,
        view_state: &mut Self::ViewState,
        ctx: &mut ViewCtx,
        element: Mut<'_, Self::Element>,
    ) {
        teardown_event_listener(
            &self.element,
            element,
            &self.event,
            view_state,
            self.capture,
            ctx,
        );
    }

    fn message(
        &self,
        view_state: &mut Self::ViewState,
        id_path: &[ViewId],
        message: crate::DynMessage,
        app_state: &mut State,
    ) -> MessageResult<Action, DynMessage> {
        message_event_listener(
            &self.element,
            view_state,
            id_path,
            message,
            app_state,
            &self.handler,
        )
    }
}

macro_rules! event_definitions {
    ($(($ty_name:ident, $event_name:literal, $web_sys_ty:ident)),*) => {
        $(
        pub struct $ty_name<V, State, Action, Callback> {
            pub(crate) element: V,
            pub(crate) capture: bool,
            pub(crate) passive: bool,
            pub(crate) handler: Callback,
            pub(crate) phantom_event_ty: PhantomData<fn() -> (State, Action)>,
        }

        impl<V, State, Action, Callback> ViewMarker for $ty_name<V, State, Action, Callback> {}
        impl<V, State, Action, Callback> $ty_name<V, State, Action, Callback> {
            pub fn new(element: V, handler: Callback) -> Self {
                Self {
                    element,
                    passive: true,
                    capture: false,
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
                self.passive = value;
                self
            }

            /// Whether the event handler should capture the event *before* being dispatched to any `EventTarget` beneath it in the DOM tree. (default = `false`)
            ///
            /// Events that are bubbling upward through the tree will not trigger a listener designated to use capture.
            /// Event bubbling and capturing are two ways of propagating events that occur in an element that is nested within another element,
            /// when both elements have registered a handle for that event.
            /// The event propagation mode determines the order in which elements receive the event.
            // TODO use similar Nomenclature as gloo (Phase::Bubble/Phase::Capture)?
            pub fn capture(mut self, value: bool) -> Self {
                self.capture = value;
                self
            }
        }


        impl<V, State, Action, Callback, OA> View<State, Action, ViewCtx, DynMessage>
            for $ty_name<V, State, Action, Callback>
        where
            State: 'static,
            Action: 'static,
            OA: OptionalAction<Action>,
            Callback: Fn(&mut State, web_sys::$web_sys_ty) -> OA + 'static,
            V: View<State, Action, ViewCtx, DynMessage>,
            V::Element: ElementAsRef<web_sys::EventTarget>,
        {
            type ViewState = OnEventState<V::ViewState>;

            type Element = V::Element;

            fn build(&self, ctx: &mut ViewCtx) -> (Self::Element, Self::ViewState) {
                build_event_listener::<_, _, _, web_sys::$web_sys_ty>(
                    &self.element,
                    $event_name,
                    self.capture,
                    self.passive,
                    ctx,
                )
            }

            fn rebuild<'el>(
                &self,
                prev: &Self,
                view_state: &mut Self::ViewState,
                ctx: &mut ViewCtx,
                element: Mut<'el, Self::Element>,
            ) -> Mut<'el, Self::Element> {
                rebuild_event_listener::<_, _, _, web_sys::$web_sys_ty>(
                    &self.element,
                    &prev.element,
                    element,
                    $event_name,
                    self.capture,
                    self.passive,
                    prev.capture,
                    prev.passive,
                    view_state,
                    ctx,
                )
            }

            fn teardown(
                &self,
                view_state: &mut Self::ViewState,
                ctx: &mut ViewCtx,
                element: Mut<'_, Self::Element>,
            ) {
                teardown_event_listener(&self.element, element, $event_name, view_state, self.capture, ctx);
            }

            fn message(
                &self,
                view_state: &mut Self::ViewState,
                id_path: &[ViewId],
                message: crate::DynMessage,
                app_state: &mut State,
            ) -> MessageResult<Action, DynMessage> {
                message_event_listener(&self.element, view_state, id_path, message, app_state, &self.handler)
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
    (OnInput, "input", Event),
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

pub struct OnResize<V, State, Action, Callback> {
    pub(crate) element: V,
    pub(crate) handler: Callback,
    pub(crate) phantom_event_ty: PhantomData<fn() -> (State, Action)>,
}

pub struct OnResizeState<VState> {
    child_state: VState,
    // Closures are retained so they can be called by environment
    #[allow(unused)]
    callback: Closure<dyn FnMut(js_sys::Array)>,
    observer: web_sys::ResizeObserver,
}

impl<V, State, Action, Callback> ViewMarker for OnResize<V, State, Action, Callback> {}
impl<State, Action, OA, Callback, V: View<State, Action, ViewCtx, DynMessage>>
    View<State, Action, ViewCtx, DynMessage> for OnResize<V, State, Action, Callback>
where
    State: 'static,
    Action: 'static,
    OA: OptionalAction<Action>,
    Callback: Fn(&mut State, web_sys::ResizeObserverEntry) -> OA + 'static,
    V: View<State, Action, ViewCtx, DynMessage>,
    V::Element: ElementAsRef<web_sys::Element>,
{
    type Element = V::Element;

    type ViewState = OnResizeState<V::ViewState>;

    fn build(&self, ctx: &mut ViewCtx) -> (Self::Element, Self::ViewState) {
        ctx.with_id(ON_EVENT_VIEW_ID, |ctx| {
            let thunk = ctx.message_thunk();
            let callback = Closure::new(move |entries: js_sys::Array| {
                let entry: web_sys::ResizeObserverEntry = entries.at(0).unchecked_into();
                thunk.push_message(entry);
            });

            let observer =
                web_sys::ResizeObserver::new(callback.as_ref().unchecked_ref()).unwrap_throw();
            let (element, child_state) = self.element.build(ctx);
            observer.observe(element.as_ref());

            let state = OnResizeState {
                child_state,
                callback,
                observer,
            };

            (element, state)
        })
    }

    fn rebuild<'el>(
        &self,
        prev: &Self,
        view_state: &mut Self::ViewState,
        ctx: &mut ViewCtx,
        element: Mut<'el, Self::Element>,
    ) -> Mut<'el, Self::Element> {
        ctx.with_id(ON_EVENT_VIEW_ID, |ctx| {
            self.element
                .rebuild(&prev.element, &mut view_state.child_state, ctx, element)
        })
    }

    fn teardown(
        &self,
        view_state: &mut Self::ViewState,
        ctx: &mut ViewCtx,
        element: Mut<'_, Self::Element>,
    ) {
        ctx.with_id(ON_EVENT_VIEW_ID, |ctx| {
            view_state.observer.unobserve(element.as_ref());
            self.element
                .teardown(&mut view_state.child_state, ctx, element);
        });
    }

    fn message(
        &self,
        view_state: &mut Self::ViewState,
        id_path: &[ViewId],
        message: DynMessage,
        app_state: &mut State,
    ) -> MessageResult<Action, DynMessage> {
        let Some((first, remainder)) = id_path.split_first() else {
            throw_str("Parent view of `OnResize` sent outdated and/or incorrect empty view path");
        };
        if *first != ON_EVENT_VIEW_ID {
            throw_str("Parent view of `OnResize` sent outdated and/or incorrect empty view path");
        }
        if remainder.is_empty() {
            let event = message
                .downcast::<web_sys::ResizeObserverEntry>()
                .unwrap_throw();
            match (self.handler)(app_state, *event).action() {
                Some(a) => MessageResult::Action(a),
                None => MessageResult::Nop,
            }
        } else {
            self.element
                .message(&mut view_state.child_state, remainder, message, app_state)
        }
    }
}
