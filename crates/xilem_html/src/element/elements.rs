//! Types that wrap [`Element`][super::Element] and represent specific element types.
//!

use std::{any::Any, borrow::Cow, collections::BTreeSet, marker::PhantomData};

use gloo::events::{EventListenerOptions, EventListenerPhase};
use wasm_bindgen::JsCast;

use super::{remove_attribute, set_attribute};
use crate::{
    diff::{diff_kv_iterables, Diff},
    event::EventMsg,
    vecmap::VecMap,
    AttributeValue, Event, IntoAttributeValue, Pod,
};
use wasm_bindgen::UnwrapThrowExt;

macro_rules! debug_warn {
    ($($arg:tt)*) => {{
        #[cfg(debug_assertions)]
        web_sys::console::warn_1(&format!($($arg)*).into());
    }}
}

type CowStr = Cow<'static, str>;
type Attributes = VecMap<CowStr, AttributeValue>;

pub trait EventHandler<T, OA, A = (), E = ()>
where
    OA: crate::OptionalAction<A>,
{
    type State;
    fn build(&self, cx: &mut crate::context::Cx) -> (xilem_core::Id, Self::State);

    fn rebuild(
        &self,
        cx: &mut crate::context::Cx,
        prev: &Self,
        id: &mut xilem_core::Id,
        state: &mut Self::State,
    ) -> crate::ChangeFlags;

    fn message(
        &self,
        id_path: &[xilem_core::Id],
        state: &mut Self::State,
        message: Box<dyn std::any::Any>,
        app_state: &mut T,
    ) -> xilem_core::MessageResult<A>;
}

impl<T, A, OA: crate::OptionalAction<A>, E: 'static, F: Fn(&mut T, E) -> OA>
    EventHandler<T, OA, A, E> for F
{
    type State = ();

    fn build(&self, _cx: &mut crate::Cx) -> (xilem_core::Id, Self::State) {
        (xilem_core::Id::next(), ())
    }

    fn rebuild(
        &self,
        _cx: &mut crate::Cx,
        _prev: &Self,
        _id: &mut xilem_core::Id,
        _state: &mut Self::State,
    ) -> crate::ChangeFlags {
        crate::ChangeFlags::empty()
    }

    fn message(
        &self,
        id_path: &[xilem_core::Id],
        _state: &mut Self::State,
        event: Box<dyn std::any::Any>,
        app_state: &mut T,
    ) -> crate::MessageResult<A> {
        if !id_path.is_empty() {
            debug_warn!("id_path isn't empty when entering event handler callback, discarding");
            return crate::MessageResult::Stale(event);
        }
        if event.downcast_ref::<EventMsg<E>>().is_some() {
            let event = *event.downcast::<EventMsg<E>>().unwrap();
            match self(app_state, event.event).action() {
                Some(a) => crate::MessageResult::Action(a),
                None => crate::MessageResult::Nop,
            }
        } else {
            debug_warn!("downcasting event in event handler callback failed, discarding");
            crate::MessageResult::Stale(event)
        }
    }
}

pub struct EventListener<T, A, OA, E, El, EH> {
    #[allow(clippy::complexity)]
    phantom: PhantomData<fn() -> (T, OA, A, E, El)>,
    event: &'static str,
    options: EventListenerOptions,
    event_handler: EH,
}

struct EventListenerState<EHS> {
    #[allow(unused)]
    listener: gloo::events::EventListener,
    handler_id: xilem_core::Id,
    handler_state: EHS,
}

impl<T, A, OA, E, El, EH> EventListener<T, A, OA, E, El, EH>
where
    E: JsCast + 'static,
    OA: crate::OptionalAction<A>,
    El: 'static,
    EH: EventHandler<T, OA, A, Event<E, El>>,
{
    fn new(event: &'static str, event_handler: EH, options: EventListenerOptions) -> Self {
        EventListener {
            phantom: PhantomData,
            event,
            options,
            event_handler,
        }
    }

    fn build(
        &self,
        cx: &mut crate::context::Cx,
        event_target: &web_sys::EventTarget,
    ) -> (xilem_core::Id, EventListenerState<EH::State>) {
        cx.with_new_id(|cx| {
            let thunk = cx.message_thunk();
            let listener = gloo::events::EventListener::new_with_options(
                event_target,
                self.event,
                self.options,
                move |event: &web_sys::Event| {
                    let event = (*event).clone().dyn_into::<E>().unwrap_throw();
                    let event: Event<E, El> = Event::new(event);
                    thunk.push_message(EventMsg { event });
                },
            );

            let (handler_id, handler_state) = self.event_handler.build(cx);

            EventListenerState {
                listener,
                handler_id,
                handler_state,
            }
        })
    }

    fn rebuild(
        &self,
        cx: &mut crate::context::Cx,
        prev: &Self,
        id: &mut xilem_core::Id,
        state: &mut EventListenerState<EH::State>,
        event_target: &web_sys::EventTarget,
    ) -> crate::ChangeFlags {
        if prev.event != self.event
            || self.options.passive != prev.options.passive
            || matches!(self.options.phase, EventListenerPhase::Bubble)
                != matches!(prev.options.phase, EventListenerPhase::Bubble)
        {
            let (new_id, new_state) = self.build(cx, event_target);
            *id = new_id;
            *state = new_state;
            crate::ChangeFlags::STRUCTURE
        } else {
            self.event_handler.rebuild(
                cx,
                &prev.event_handler,
                &mut state.handler_id,
                &mut state.handler_state,
            )
        }
    }

    fn message(
        &self,
        id_path: &[xilem_core::Id],
        state: &mut EventListenerState<EH::State>,
        message: Box<dyn std::any::Any>,
        app_state: &mut T,
    ) -> xilem_core::MessageResult<A> {
        if id_path.is_empty() {
            return self
                .event_handler
                .message(&[], &mut state.handler_state, message, app_state);
        } else if let Some((first, rest_path)) = id_path.split_first() {
            if *first == state.handler_id {
                return self.event_handler.message(
                    rest_path,
                    &mut state.handler_state,
                    message,
                    app_state,
                );
            }
        }
        xilem_core::MessageResult::Stale(message)
    }
}

type DynamicEventListenerBuildFn<T, A> = fn(
    &DynamicEventListener<T, A>,
    &web_sys::EventTarget,
    &mut crate::Cx,
) -> (xilem_core::Id, Box<dyn Any>);

type DynamicEventListenerRebuildFn<T, A> = fn(
    &DynamicEventListener<T, A>,
    &web_sys::EventTarget,
    &mut crate::context::Cx,
    &DynamicEventListener<T, A>,
    &mut xilem_core::Id,
    &mut Box<dyn Any>,
) -> crate::ChangeFlags;

type DynamicEventListenerMessageFn<T, A> = fn(
    &DynamicEventListener<T, A>,
    &[xilem_core::Id],
    &mut dyn Any,
    Box<dyn Any>,
    &mut T,
) -> xilem_core::MessageResult<A>;

struct DynamicEventListener<T, A> {
    listener: Box<dyn Any>,
    build: DynamicEventListenerBuildFn<T, A>,
    rebuild: DynamicEventListenerRebuildFn<T, A>,
    message: DynamicEventListenerMessageFn<T, A>,
}

impl<T, A> DynamicEventListener<T, A> {
    pub fn new<OA, E, EL, EH>(listener: EventListener<T, A, OA, E, EL, EH>) -> Self
    where
        T: 'static,
        A: 'static,
        E: JsCast + 'static,
        OA: crate::OptionalAction<A> + 'static,
        EL: 'static,
        EH: EventHandler<T, OA, A, Event<E, EL>> + 'static,
    {
        let build: DynamicEventListenerBuildFn<T, A> = |self_, element, cx| {
            let (id, state) = self_
                .listener
                .downcast_ref::<EventListener<T, A, OA, E, EL, EH>>()
                .unwrap()
                .build(cx, element);
            (id, Box::new(state))
        };

        let rebuild: DynamicEventListenerRebuildFn<T, A> =
            |self_, event_target, cx, prev, id, state| {
                let listener = self_
                    .listener
                    .downcast_ref::<EventListener<T, A, OA, E, EL, EH>>()
                    .unwrap();
                if let Some(prev) = prev.listener.downcast_ref() {
                    if let Some(state) = state.downcast_mut() {
                        listener.rebuild(cx, prev, id, state, event_target)
                    } else {
                        debug_warn!("downcasting state for event '{}' failed", listener.event);
                        crate::ChangeFlags::default()
                    }
                } else {
                    let (new_id, new_state) = self_.build(event_target, cx);
                    *id = new_id;
                    *state = Box::new(new_state);
                    crate::ChangeFlags::STRUCTURE
                }
            };

        let message: DynamicEventListenerMessageFn<T, A> =
            |self_, id_path, state, message, app_state| {
                let listener = self_
                    .listener
                    .downcast_ref::<EventListener<T, A, OA, E, EL, EH>>()
                    .unwrap();
                if let Some(state) = state.downcast_mut() {
                    listener.message(id_path, state, message, app_state)
                } else {
                    debug_warn!(
                        "message/event downcasting for event '{}' failed",
                        listener.event
                    );
                    xilem_core::MessageResult::Stale(message)
                }
            };
        Self {
            listener: Box::new(listener),
            build,
            rebuild,
            message,
        }
    }

    pub fn build(
        &self,
        element: &web_sys::EventTarget,
        cx: &mut crate::context::Cx,
    ) -> (xilem_core::Id, Box<dyn Any>) {
        (self.build)(self, element, cx)
    }

    pub fn rebuild(
        &self,
        event_target: &web_sys::EventTarget,
        cx: &mut crate::context::Cx,
        prev: &Self,
        id: &mut xilem_core::Id,
        state: &mut Box<dyn Any>,
    ) -> crate::ChangeFlags {
        (self.rebuild)(self, event_target, cx, prev, id, state)
    }

    pub fn message(
        &self,
        id_path: &[xilem_core::Id],
        state: &mut dyn Any,
        message: Box<dyn Any>,
        app_state: &mut T,
    ) -> xilem_core::MessageResult<A> {
        (self.message)(self, id_path, state, message, app_state)
    }
}

type EventListenersState = Vec<(xilem_core::Id, Box<dyn Any>)>;

pub struct ElementState<ViewSeqState> {
    children_states: ViewSeqState,
    children_elements: Vec<Pod>,
    event_listener_state: EventListenersState,
    scratch: Vec<Pod>,
}

fn impl_build_element<T, A>(
    cx: &mut crate::context::Cx,
    id: xilem_core::Id,
    node_name: &str,
    attrs: &Attributes,
    children: &Vec<Pod>,
    event_listeners: &[DynamicEventListener<T, A>],
) -> (web_sys::HtmlElement, EventListenersState) {
    cx.with_id(id, |cx| {
        let el = cx.create_html_element(node_name);

        for (name, value) in attrs.iter() {
            el.set_attribute(name, &value.serialize()).unwrap_throw();
        }

        for child in children {
            el.append_child(child.0.as_node_ref()).unwrap_throw();
        }

        let event_listener_state = event_listeners
            .iter()
            .map(|listener| listener.build(&el, cx))
            .collect();

        // Set the id used internally to the `data-debugid` attribute.
        // This allows the user to see if an element has been re-created or only altered.
        #[cfg(debug_assertions)]
        el.set_attribute("data-debugid", &id.to_raw().to_string())
            .unwrap_throw();

        (el, event_listener_state)
    })
}

#[allow(clippy::too_many_arguments)]
fn impl_rebuild_element<T, A>(
    cx: &mut crate::context::Cx,
    attrs: &Attributes,
    prev_attrs: &Attributes,
    element: &web_sys::Element,
    prev_event_listeners: &[DynamicEventListener<T, A>],
    event_listeners: &[DynamicEventListener<T, A>],
    event_listeners_state: &mut EventListenersState,
    mut children_changed: crate::ChangeFlags,
    children: &[Pod],
) -> crate::ChangeFlags {
    use crate::ChangeFlags;
    let mut changed = ChangeFlags::empty();

    // diff attributes
    for itm in diff_kv_iterables(prev_attrs, attrs) {
        match itm {
            Diff::Add(name, value) | Diff::Change(name, value) => {
                set_attribute(element, name, &value.serialize());
                changed |= ChangeFlags::OTHER_CHANGE;
            }
            Diff::Remove(name) => {
                remove_attribute(element, name);
                changed |= ChangeFlags::OTHER_CHANGE;
            }
        }
    }

    if children_changed.contains(ChangeFlags::STRUCTURE) {
        // This is crude and will result in more DOM traffic than needed.
        // The right thing to do is diff the new state of the children id
        // vector against the old, and derive DOM mutations from that.
        while let Some(child) = element.first_child() {
            element.remove_child(&child).unwrap_throw();
        }
        for child in children {
            element.append_child(child.0.as_node_ref()).unwrap_throw();
        }
        children_changed.remove(ChangeFlags::STRUCTURE);
    }

    for ((listener, listener_prev), (listener_id, listener_state)) in event_listeners
        .iter()
        .zip(prev_event_listeners.iter())
        .zip(event_listeners_state.iter_mut())
    {
        let listener_changed =
            listener.rebuild(element, cx, listener_prev, listener_id, listener_state);
        changed |= listener_changed;
    }

    let cur_listener_len = event_listeners.len();
    let state_len = event_listeners_state.len();

    #[allow(clippy::comparison_chain)]
    if cur_listener_len < state_len {
        event_listeners_state.truncate(cur_listener_len);
        changed |= ChangeFlags::STRUCTURE;
    } else if cur_listener_len > state_len {
        for listener in &event_listeners[state_len..cur_listener_len] {
            event_listeners_state.push(listener.build(element, cx));
        }
        changed |= ChangeFlags::STRUCTURE;
    }
    changed | children_changed
}

fn impl_message_element<T, A>(
    id_path: &[xilem_core::Id],
    event_listeners: &[DynamicEventListener<T, A>],
    event_listeners_state: &mut EventListenersState,
    message: Box<dyn Any>,
    app_state: &mut T,
) -> xilem_core::MessageResult<A> {
    if let Some((first, rest_path)) = id_path.split_first() {
        if let Some((idx, (_, listener_state))) = event_listeners_state
            .iter_mut()
            .enumerate()
            .find(|(_, (id, _))| id == first)
        {
            let listener = &event_listeners[idx];
            return listener.message(rest_path, listener_state.as_mut(), message, app_state);
        }
    }
    xilem_core::MessageResult::Stale(message)
}

pub trait Node {
    fn node_name(&self) -> &str;
}

macro_rules! def_simple_attr {
    ($name:ident, $setter_name: ident, $ty: ty) => {
        #[inline(always)]
        fn $name(mut self, $name: $ty) -> Self {
            self.$setter_name($name);
            self
        }

        #[inline(always)]
        fn $setter_name(&mut self, $name: $ty) {
            let value = $name.into_attribute_value().unwrap();
            self.attrs_mut().insert(stringify!($name).into(), value);
        }
    };
}

// TODO add also something like `set_on_click(&mut self)`?
macro_rules! def_event_attr {
    ($name: ident, $name_with_options: ident, $event: ty, $event_name: expr) => {
        fn $name<EH, OA>(self, handler: EH) -> Self
        where
            T: 'static,
            A: 'static,
            Self::Element: 'static,
            OA: crate::OptionalAction<A> + 'static,
            EH: EventHandler<T, OA, A, crate::Event<$event, Self::Element>> + 'static,
        {
            self.$name_with_options(handler, EventListenerOptions::default())
        }

        fn $name_with_options<EH, OA>(mut self, handler: EH, options: EventListenerOptions) -> Self
        where
            T: 'static,
            A: 'static,
            Self::Element: 'static,
            OA: crate::OptionalAction<A> + 'static,
            EH: EventHandler<T, OA, A, crate::Event<$event, Self::Element>> + 'static,
        {
            self.add_event_listener(EventListener::new($event_name, handler, options));
            self
        }
    };
}

/// These traits should mirror the respective DOM interfaces
/// In this case https://dom.spec.whatwg.org/#interface-element
/// Or rather a curated/opinionated subset that makes sense in xilem for each of these interfaces
/// unfortunately with this (builder + generic type parameters in methods) pattern not trait-object-safe
pub trait Element<T, A>: Node + crate::view::View<T, A> + Sized {
    // The following three functions are the only ones that need to be implemented by each element.
    // All other functions are implemented either in this trait or in the corresponding dom interface traits
    /// Return the raw `&Attributes` of this element
    fn attrs(&self) -> &Attributes;
    /// Return the raw `&mut Attributes` of this element
    fn attrs_mut(&mut self) -> &mut Attributes;

    fn add_event_listener<E, OA, EH>(
        &mut self,
        listener: EventListener<T, A, OA, E, Self::Element, EH>,
    ) where
        T: 'static,
        A: 'static,
        E: JsCast + 'static,
        OA: crate::OptionalAction<A> + 'static,
        EH: EventHandler<T, OA, A, crate::Event<E, Self::Element>> + 'static;

    fn on<E, OA, EH>(mut self, event: &'static str, handler: EH) -> Self
    where
        T: 'static,
        A: 'static,
        Self::Element: 'static,
        E: JsCast + 'static,
        OA: crate::OptionalAction<A> + 'static,
        EH: EventHandler<T, OA, A, crate::Event<E, Self::Element>> + 'static,
    {
        self.add_event_listener(EventListener::new(event, handler, Default::default()));
        self
    }

    fn class<C: IntoClass>(mut self, class: C) -> Self {
        add_class(self.attrs_mut(), class);
        self
    }

    fn add_class<C: IntoClass>(&mut self, class: C) {
        add_class(self.attrs_mut(), class);
    }

    /// Set an attribute on this element.
    ///
    /// # Panics
    ///
    /// If the name contains characters that are not valid in an attribute name,
    /// then the `View::build`/`View::rebuild` functions will panic for this view.
    fn attr<K: Into<CowStr>, V: IntoAttributeValue>(mut self, key: K, value: V) -> Self {
        self.set_attr(key, value);
        self
    }

    /// Set an attribute on this element.
    ///
    /// # Panics
    ///
    /// If the name contains characters that are not valid in an attribute name,
    /// then the `View::build`/`View::rebuild` functions will panic for this view.
    fn set_attr<K: Into<CowStr>, V: IntoAttributeValue>(&mut self, key: K, value: V) {
        let key = key.into();
        if let Some(value) = value.into_attribute_value() {
            self.attrs_mut().insert(key, value);
        } else {
            self.attrs_mut().remove(&key);
        }
    }

    // Mouse events
    def_event_attr!(
        on_click,
        on_click_with_options,
        web_sys::MouseEvent,
        "click"
    );
    def_event_attr!(
        on_dblclick,
        on_dblclick_with_options,
        web_sys::MouseEvent,
        "dblclick"
    );
    def_event_attr!(
        on_auxclick,
        on_auxclick_with_options,
        web_sys::MouseEvent,
        "auxclick"
    );
    def_event_attr!(
        on_contextmenu,
        on_contextmenu_with_options,
        web_sys::MouseEvent,
        "contextmenu"
    );
    def_event_attr!(
        on_mousedown,
        on_mousedown_with_options,
        web_sys::MouseEvent,
        "mousedown"
    );
    def_event_attr!(
        on_mouseenter,
        on_mouseenter_with_options,
        web_sys::MouseEvent,
        "mouseenter"
    );
    def_event_attr!(
        on_mouseleave,
        on_mouseleave_with_options,
        web_sys::MouseEvent,
        "mouseleave"
    );
    def_event_attr!(
        on_mousemove,
        on_mousemove_with_options,
        web_sys::MouseEvent,
        "mousemove"
    );
    def_event_attr!(
        on_mouseout,
        on_mouseout_with_options,
        web_sys::MouseEvent,
        "mouseout"
    );
    def_event_attr!(
        on_mouseover,
        on_mouseover_with_options,
        web_sys::MouseEvent,
        "mouseover"
    );
    def_event_attr!(
        on_mouseup,
        on_mouseup_with_options,
        web_sys::MouseEvent,
        "mouseup"
    );

    // Scroll events
    def_event_attr!(on_scroll, on_scroll_with_options, web_sys::Event, "scroll");
    def_event_attr!(
        on_scrollend,
        on_scrollend_with_options,
        web_sys::Event,
        "scrollend"
    );

    // Keyboard events
    def_event_attr!(
        on_keydown,
        on_keydown_with_options,
        web_sys::KeyboardEvent,
        "keydown"
    );
    def_event_attr!(
        on_keyup,
        on_keyup_with_options,
        web_sys::KeyboardEvent,
        "keyup"
    );

    // Focus events
    def_event_attr!(
        on_focus,
        on_focus_with_options,
        web_sys::FocusEvent,
        "focus"
    );
    def_event_attr!(
        on_focusin,
        on_focusin_with_options,
        web_sys::FocusEvent,
        "focusin"
    );
    def_event_attr!(
        on_focusout,
        on_focusout_with_options,
        web_sys::FocusEvent,
        "focusout"
    );
    def_event_attr!(on_blur, on_blur_with_options, web_sys::FocusEvent, "blur");

    def_event_attr!(
        on_input,
        on_input_with_options,
        web_sys::InputEvent,
        "input"
    );

    // With an explicit Fn traitbound for the event-handler/callback, it's possible to workaround explicit necessary typing
    // But this obviously restricts the event handler to this exact function type
    fn on_touchstart<F, OA>(mut self, callback: F) -> Self
    where
        T: 'static,
        A: 'static,
        Self::Element: 'static,
        OA: crate::OptionalAction<A> + 'static,
        F: for<'a> Fn(&'a mut T, crate::Event<web_sys::TouchEvent, Self::Element>) -> OA + 'static,
    {
        self.add_event_listener(EventListener::new(
            "touchstart",
            callback,
            Default::default(),
        ));
        self
    }

    // TODO rest of all the methods allowed on an element
}

macro_rules! dom_interface_trait_definitions {
    ($($dom_interface:ident : $super_dom_interface: ident $body: tt),*) => {
        $(pub trait $dom_interface<T, A>: $super_dom_interface<T, A> $body)*
    };
}

// TODO all the typed attributes
dom_interface_trait_definitions!(
    HtmlAnchorElement : HtmlElement {},
    HtmlAreaElement : HtmlElement {},
    HtmlAudioElement : HtmlMediaElement {},
    HtmlBaseElement : HtmlElement {},
    HtmlBodyElement : HtmlElement {},
    HtmlBrElement : HtmlElement {},
    HtmlButtonElement : HtmlElement {},
    HtmlCanvasElement : HtmlElement {
        def_simple_attr!(width, set_width, u32);
        def_simple_attr!(height, set_height, u32);
    },
    HtmlDataElement : HtmlElement {},
    HtmlDataListElement : HtmlElement {},
    HtmlDetailsElement : HtmlElement {},
    HtmlDialogElement : HtmlElement {},
    HtmlDirectoryElement : HtmlElement {},
    HtmlDivElement : HtmlElement {},
    HtmlDListElement : HtmlElement {},
    HtmlElement : Element {},
    HtmlUnknownElement : HtmlElement {},
    HtmlEmbedElement : HtmlElement {},
    HtmlFieldSetElement : HtmlElement {},
    HtmlFontElement : HtmlElement {},
    HtmlFormElement : HtmlElement {},
    HtmlFrameElement : HtmlElement {},
    HtmlFrameSetElement : HtmlElement {},
    HtmlHeadElement : HtmlElement {},
    HtmlHeadingElement : HtmlElement {},
    HtmlHrElement : HtmlElement {},
    HtmlHtmlElement : HtmlElement {},
    HtmlIFrameElement : HtmlElement {},
    HtmlImageElement : HtmlElement {},
    HtmlInputElement : HtmlElement {},
    HtmlLabelElement : HtmlElement {},
    HtmlLegendElement : HtmlElement {},
    HtmlLiElement : HtmlElement {},
    HtmlLinkElement : HtmlElement {},
    HtmlMapElement : HtmlElement {},
    HtmlMediaElement : HtmlElement {},
    HtmlMenuElement : HtmlElement {},
    HtmlMenuItemElement : HtmlElement {},
    HtmlMetaElement : HtmlElement {},
    HtmlMeterElement : HtmlElement {},
    HtmlModElement : HtmlElement {},
    HtmlObjectElement : HtmlElement {},
    HtmlOListElement : HtmlElement {},
    HtmlOptGroupElement : HtmlElement {},
    HtmlOptionElement : HtmlElement {},
    HtmlOutputElement : HtmlElement {},
    HtmlParagraphElement : HtmlElement {},
    HtmlParamElement : HtmlElement {},
    HtmlPictureElement : HtmlElement {},
    HtmlPreElement : HtmlElement {},
    HtmlProgressElement : HtmlElement {},
    HtmlQuoteElement : HtmlElement {},
    HtmlScriptElement : HtmlElement {},
    HtmlSelectElement : HtmlElement {},
    HtmlSlotElement : HtmlElement {},
    HtmlSourceElement : HtmlElement {},
    HtmlSpanElement : HtmlElement {},
    HtmlStyleElement : HtmlElement {},
    HtmlTableCaptionElement : HtmlElement {},
    HtmlTableCellElement : HtmlElement {},
    HtmlTableColElement : HtmlElement {},
    HtmlTableElement : HtmlElement {},
    HtmlTableRowElement : HtmlElement {},
    HtmlTableSectionElement : HtmlElement {},
    HtmlTemplateElement : HtmlElement {},
    HtmlTimeElement : HtmlElement {},
    HtmlTextAreaElement : HtmlElement {},
    HtmlTitleElement : HtmlElement {},
    HtmlTrackElement : HtmlElement {},
    HtmlUListElement : HtmlElement {},
    HtmlVideoElement : HtmlMediaElement {}
);

fn add_class<C: IntoClass>(attrs: &mut Attributes, class: C) {
    let mut class = class.into_class().peekable();

    if class.peek().is_none() {
        return;
    }

    match attrs.get_mut("class") {
        Some(AttributeValue::Classes(attr_value)) => {
            attr_value.extend(class);
        }
        // could be useful, in case untyped values are inserted here
        Some(untyped_class) if matches!(untyped_class, AttributeValue::String(_)) => {
            let mut class = BTreeSet::from_iter(class);
            class.insert(if let AttributeValue::String(s) = untyped_class {
                s.clone()
            } else {
                unreachable!()
            });
            *untyped_class = AttributeValue::Classes(class);
        }
        Some(other) => {
            // TODO warning
            // panic!("A static attribute 'class' should always have either the type BTreeSet<CowStr> or String")
            *other = AttributeValue::Classes(BTreeSet::from_iter(class));
        }
        None => {
            attrs.insert(
                "class".into(),
                AttributeValue::Classes(BTreeSet::from_iter(class)),
            );
        }
    };
}

// Since these methods are used for all HTML elements,
// it might make sense to add an extra inner impl function if possible
// (see below at `simple_attr_impl` for an example) to avoid big compilation code size
macro_rules! impl_element {
    ($ty_name:ident, $name: ident, $concrete_dom_interface: ident) => {
        impl<T_, A_, VS> Element<T_, A_> for $ty_name<T_, A_, VS>
        where
            VS: crate::view::ViewSequence<T_, A_>,
        {
            fn attrs(&self) -> &Attributes {
                &self.attrs
            }

            fn attrs_mut(&mut self) -> &mut Attributes {
                &mut self.attrs
            }

            fn add_event_listener<E, OA, EH>(
                &mut self,
                listener: EventListener<T_, A_, OA, E, Self::Element, EH>,
            ) where
                T_: 'static,
                A_: 'static,
                E: JsCast + 'static,
                OA: crate::OptionalAction<A_> + 'static,
                EH: EventHandler<T_, OA, A_, crate::Event<E, Self::Element>> + 'static,
            {
                self.event_listeners
                    .push(DynamicEventListener::new(listener));
            }
        }
    };
}

macro_rules! generate_dom_interface_impl {
    ($ty_name:ident, $name:ident, $dom_interface:ident) => {
        generate_dom_interface_impl!($ty_name, $name, $dom_interface, {});
    };
    ($ty_name:ident, $name:ident, $dom_interface:ident, $body: tt) => {
        impl<T_, A_, VS> $dom_interface<T_, A_> for $ty_name<T_, A_, VS>
        where
            VS: crate::view::ViewSequence<T_, A_>,
        $body
    };
}

macro_rules! impl_html_dom_interface {
    ($ty_name: ident, $name: ident, Node) => {
        impl<T_, A_, VS> Node for $ty_name<T_, A_, VS> {
            fn node_name(&self) -> &str {
                stringify!($name)
            }
        }
    };
    ($ty_name: ident, $name: ident, $concrete_dom_interface: ident, Element) => {
        impl_html_dom_interface!($ty_name, $name, Node);
        impl_element!($ty_name, $name, $concrete_dom_interface);
    };
    ($ty_name: ident, $name: ident, $concrete_dom_interface: ident, HtmlElement) => {
        impl_html_dom_interface!($ty_name, $name, $concrete_dom_interface, Element);
        generate_dom_interface_impl!($ty_name, $name, HtmlElement);
    };
    ($ty_name: ident, $name: ident, $concrete_dom_interface: ident, HtmlAudioElement) => {
        impl_html_dom_interface!($ty_name, $name, $concrete_dom_interface, HtmlMediaElement);
        generate_dom_interface_impl!($ty_name, $name, HtmlHeadingElement);
    };
    ($ty_name: ident, $name: ident, $concrete_dom_interface: ident, HtmlVideoElement) => {
        impl_html_dom_interface!($ty_name, $name, $concrete_dom_interface, HtmlMediaElement);
        generate_dom_interface_impl!($ty_name, $name, HtmlHeadingElement);
    };
    // TODO resolve parent interface correctly
    // All remaining interfaces inherit directly from HtmlElement
    ($ty_name: ident, $name: ident, $concrete_dom_interface: ident, $dom_interface: ident) => {
        impl_html_dom_interface!($ty_name, $name, $concrete_dom_interface, HtmlElement);
        generate_dom_interface_impl!($ty_name, $name, $dom_interface);
    };
}

// TODO only use T_ A_ when necessary (e.g. for the `A` element)
// TODO maybe it's possible to reduce even more in the impl function bodies and put into impl_functions
//      (should improve compile times and probably wasm binary size)
macro_rules! define_html_elements {
    ($(($ty_name:ident, $name:ident, $dom_interface:ident),)*) => {
        $(
        // TODO not sure how much it helps reducing the code size,
        // but the two attributes could be extracted into its own type, and the actual element type is just a single tuple struct wrapping this type,
        pub struct $ty_name<T_, A_, VS> {
            pub(crate) attrs: Attributes,
            event_listeners: Vec<DynamicEventListener<T_, A_>>,
            children: VS,
            phantom: std::marker::PhantomData<fn() -> (T_, A_)>,
        }

        impl<T_, A_, VS> crate::view::ViewMarker for $ty_name<T_, A_, VS> {}

        impl<T_, A_, VS> crate::view::View<T_, A_> for $ty_name<T_, A_, VS>
        where
            VS: crate::view::ViewSequence<T_, A_>,
        {
            type State = ElementState<VS::State>;
            type Element = web_sys::$dom_interface;

            fn build(&self, cx: &mut crate::context::Cx) -> (xilem_core::Id, Self::State, Self::Element) {
                // TODO remove
                // debug_log!("new element built: {}", self.node_name());

                let mut children_elements = vec![];
                let (id, children_states) =
                    cx.with_new_id(|cx| self.children.build(cx, &mut children_elements));
                let (el, event_listener_state) = impl_build_element(
                    cx,
                    id,
                    self.node_name(),
                    &self.attrs,
                    &children_elements,
                    &self.event_listeners,
                );

                let state = ElementState {
                    children_states,
                    children_elements,
                    event_listener_state,
                    scratch: vec![],
                };
                use wasm_bindgen::UnwrapThrowExt;
                let el = el.dyn_into().unwrap_throw();
                (id, state, el)
            }

            fn rebuild(
                &self,
                cx: &mut crate::context::Cx,
                prev: &Self,
                id: &mut xilem_core::Id,
                state: &mut Self::State,
                element: &mut Self::Element,
            ) -> crate::ChangeFlags {
                debug_assert!(prev.node_name() == self.node_name());

                cx.with_id(*id, |cx| {
                    let mut splice =
                        xilem_core::VecSplice::new(&mut state.children_elements, &mut state.scratch);
                    let children_changed =
                        self.children
                            .rebuild(cx, &prev.children, &mut state.children_states, &mut splice);
                    impl_rebuild_element(
                        cx,
                        &self.attrs,
                        &prev.attrs,
                        element,
                        &prev.event_listeners,
                        &self.event_listeners,
                        &mut state.event_listener_state,
                        children_changed,
                        &state.children_elements,
                    )
                })
            }

            fn message(
                &self,
                id_path: &[xilem_core::Id],
                state: &mut Self::State,
                message: Box<dyn std::any::Any>,
                app_state: &mut T_,
            ) -> xilem_core::MessageResult<A_> {
                debug_assert!(state.event_listener_state.len() == self.event_listeners.len());
                impl_message_element(
                    id_path,
                    &self.event_listeners,
                    &mut state.event_listener_state,
                    message,
                    app_state,
                )
                .or(|message| {
                    self.children
                        .message(id_path, &mut state.children_states, message, app_state)
                })
            }
        }


        /// Builder function for a
        #[doc = concat!("`", stringify!($name), "`")]
        /// element view.
        pub fn $name<T_, A_, VS>(children: VS) -> $ty_name<T_, A_, VS>
        where
            VS: crate::view::ViewSequence<T_, A_>,
        {
            $ty_name {
                attrs: Default::default(),
                children,
                phantom: std::marker::PhantomData,
                event_listeners: Default::default(),
            }
        }

        impl_html_dom_interface!($ty_name, $name, $dom_interface, $dom_interface);
        )*
    };
}

define_html_elements!(
    // the order is copied from
    // https://developer.mozilla.org/en-US/docs/Web/HTML/Element
    // DOM interfaces copied from https://html.spec.whatwg.org/multipage/grouping-content.html and friends

    // content sectioning
    (Address, address, HtmlElement),
    (Article, article, HtmlElement),
    (Aside, aside, HtmlElement),
    (Footer, footer, HtmlElement),
    (Header, header, HtmlElement),
    (H1, h1, HtmlHeadingElement),
    (H2, h2, HtmlHeadingElement),
    (H3, h3, HtmlHeadingElement),
    (H4, h4, HtmlHeadingElement),
    (H5, h5, HtmlHeadingElement),
    (H6, h6, HtmlHeadingElement),
    (Hgroup, hgroup, HtmlElement),
    (Main, main, HtmlElement),
    (Nav, nav, HtmlElement),
    (Section, section, HtmlElement),
    // text content
    (Blockquote, blockquote, HtmlQuoteElement),
    (Dd, dd, HtmlElement),
    (Div, div, HtmlDivElement),
    (Dl, dl, HtmlDListElement),
    (Dt, dt, HtmlElement),
    (Figcaption, figcaption, HtmlElement),
    (Figure, figure, HtmlElement),
    (Hr, hr, HtmlHrElement),
    (Li, li, HtmlLiElement),
    (Menu, menu, HtmlMenuElement),
    (Ol, ol, HtmlOListElement),
    (P, p, HtmlParagraphElement),
    (Pre, pre, HtmlPreElement),
    (Ul, ul, HtmlUListElement),
    // inline text
    (A, a, HtmlAnchorElement),
    (Abbr, abbr, HtmlElement),
    (B, b, HtmlElement),
    (Bdi, bdi, HtmlElement),
    (Bdo, bdo, HtmlElement),
    (Br, br, HtmlBrElement),
    (Cite, cite, HtmlElement),
    (Code, code, HtmlElement),
    (Data, data, HtmlDataElement),
    (Dfn, dfn, HtmlElement),
    (Em, em, HtmlElement),
    (I, i, HtmlElement),
    (Kbd, kbd, HtmlElement),
    (Mark, mark, HtmlElement),
    (Q, q, HtmlQuoteElement),
    (Rp, rp, HtmlElement),
    (Rt, rt, HtmlElement),
    (Ruby, ruby, HtmlElement),
    (S, s, HtmlElement),
    (Samp, samp, HtmlElement),
    (Small, small, HtmlElement),
    (Span, span, HtmlSpanElement),
    (Strong, strong, HtmlElement),
    (Sub, sub, HtmlElement),
    (Sup, sup, HtmlElement),
    (Time, time, HtmlTimeElement),
    (U, u, HtmlElement),
    (Var, var, HtmlElement),
    (Wbr, wbr, HtmlElement),
    // image and multimedia
    (Area, area, HtmlAreaElement),
    (Audio, audio, HtmlAudioElement),
    (Img, img, HtmlImageElement),
    (Map, map, HtmlMapElement),
    (Track, track, HtmlTrackElement),
    (Video, video, HtmlVideoElement),
    // embedded content
    (Embed, embed, HtmlEmbedElement),
    (Iframe, iframe, HtmlIFrameElement),
    (Object, object, HtmlObjectElement),
    (Picture, picture, HtmlPictureElement),
    (Portal, portal, HtmlElement),
    (Source, source, HtmlSourceElement),
    // SVG and MathML (TODO, svg and mathml elements)
    (Svg, svg, HtmlElement),
    (Math, math, HtmlElement),
    // scripting
    (Canvas, canvas, HtmlCanvasElement),
    (Noscript, noscript, HtmlElement),
    (Script, script, HtmlScriptElement),
    // demarcating edits
    (Del, del, HtmlModElement),
    (Ins, ins, HtmlModElement),
    // tables
    (Caption, caption, HtmlTableCaptionElement),
    (Col, col, HtmlTableColElement),
    (Colgroup, colgroup, HtmlTableColElement),
    (Table, table, HtmlTableSectionElement),
    (Tbody, tbody, HtmlTableSectionElement),
    (Td, td, HtmlTableCellElement),
    (Tfoot, tfoot, HtmlTableSectionElement),
    (Th, th, HtmlTableCellElement),
    (Thead, thead, HtmlTableSectionElement),
    (Tr, tr, HtmlTableRowElement),
    // forms
    (Button, button, HtmlButtonElement),
    (Datalist, datalist, HtmlDataListElement),
    (Fieldset, fieldset, HtmlFieldSetElement),
    (Form, form, HtmlFormElement),
    (Input, input, HtmlInputElement),
    (Label, label, HtmlLabelElement),
    (Legend, legend, HtmlLegendElement),
    (Meter, meter, HtmlMeterElement),
    (Optgroup, optgroup, HtmlOptGroupElement),
    (OptionElement, option, HtmlOptionElement), // Avoid cluttering the namespace with `Option`
    (Output, output, HtmlOutputElement),
    (Progress, progress, HtmlProgressElement),
    (Select, select, HtmlSelectElement),
    (Textarea, textarea, HtmlTextAreaElement),
    // interactive elements,
    (Details, details, HtmlDetailsElement),
    (Dialog, dialog, HtmlDialogElement),
    (Summary, summary, HtmlElement),
    // web components,
    (Slot, slot, HtmlSlotElement),
    (Template, template, HtmlTemplateElement),
);

// A few experiments for more flexible attributes (el.class<C: IntoClass>(class: C))
pub trait IntoClass {
    type ClassIter: Iterator<Item = CowStr>;
    fn into_class(self) -> Self::ClassIter;
}

impl IntoClass for &'static str {
    type ClassIter = std::option::IntoIter<CowStr>;
    fn into_class(self) -> Self::ClassIter {
        Some(self.into()).into_iter()
    }
}

impl IntoClass for String {
    type ClassIter = std::option::IntoIter<CowStr>;
    fn into_class(self) -> Self::ClassIter {
        Some(self.into()).into_iter()
    }
}

impl IntoClass for CowStr {
    type ClassIter = std::option::IntoIter<CowStr>;
    fn into_class(self) -> Self::ClassIter {
        Some(self).into_iter()
    }
}

impl<T: IntoClass, const N: usize> IntoClass for [T; N] {
    // we really need impl
    type ClassIter =
        std::iter::FlatMap<std::array::IntoIter<T, N>, T::ClassIter, fn(T) -> T::ClassIter>;
    fn into_class(self) -> Self::ClassIter {
        self.into_iter().flat_map(IntoClass::into_class)
    }
}

impl<'a> IntoClass for &'a [&'static str] {
    // we really need impl
    type ClassIter = std::iter::Map<
        std::iter::Copied<std::slice::Iter<'a, &'static str>>,
        fn(&'static str) -> CowStr,
    >;
    fn into_class(self) -> Self::ClassIter {
        self.iter().copied().map(Cow::from)
    }
}

impl<T: IntoClass> IntoClass for Option<T> {
    // we really need impl
    type ClassIter =
        std::iter::FlatMap<std::option::IntoIter<T>, T::ClassIter, fn(T) -> T::ClassIter>;
    fn into_class(self) -> Self::ClassIter {
        self.into_iter().flat_map(IntoClass::into_class)
    }
}

impl<T: IntoClass> IntoClass for Vec<T> {
    type ClassIter = std::iter::FlatMap<std::vec::IntoIter<T>, T::ClassIter, fn(T) -> T::ClassIter>;
    fn into_class(self) -> Self::ClassIter {
        self.into_iter().flat_map(IntoClass::into_class)
    }
}

// TODO some type-fu to get something like this working:
// impl<T: IntoClass, I: IntoIterator<Item = T>> IntoClass for I {
//     type ClassIter = ...;
//     fn classes(self) -> Self::ClassIter {
//         self.into_iter().flat_map(IntoClass::classes)
//     }
// }

// TODO do we want to use the tuple syntax here ("conflicts" with ViewSequence)?
// It allows different types for each tuple member though, which might be useful,
// but an alternative would be multiple class invocations with different types
impl<A: IntoClass, B: IntoClass> IntoClass for (A, B) {
    type ClassIter = std::iter::Chain<A::ClassIter, B::ClassIter>;
    fn into_class(self) -> Self::ClassIter {
        self.0.into_class().chain(self.1.into_class())
    }
}
