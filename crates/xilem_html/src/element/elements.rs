//! Types that wrap [`Element`][super::Element] and represent specific element types.
//!

use std::{
    any::Any,
    borrow::{Borrow, Cow},
    cmp::Ordering,
    collections::{BTreeMap, BTreeSet},
    iter::Peekable,
    marker::PhantomData,
};

use gloo::events::{EventListenerOptions, EventListenerPhase};
use wasm_bindgen::JsCast;

use crate::{event::EventMsg, Event, Pod};
use wasm_bindgen::UnwrapThrowExt;

macro_rules! debug_warn {
    ($($arg:tt)*) => {{
        #[cfg(debug_assertions)]
        web_sys::console::warn_1(&format!($($arg)*).into());
    }}
}

use super::{remove_attribute, set_attribute};
macro_rules! elements {
    () => {};
    (($ty_name:ident, $name:ident, $web_sys_ty:ty), $($rest:tt)*) => {
        element!($ty_name, $name, $web_sys_ty);
        elements!($($rest)*);
    };
}

macro_rules! element {
    ($ty_name:ident, $name:ident, $web_sys_ty:ty) => {
        /// A view representing a
        #[doc = concat!("`", stringify!($name), "`")]
        /// element.
        pub struct $ty_name<ViewSeq>(crate::Element<$web_sys_ty, ViewSeq>);

        /// Builder function for a
        #[doc = concat!("`", stringify!($name), "`")]
        /// view.
        pub fn $name<ViewSeq>(children: ViewSeq) -> $ty_name<ViewSeq> {
            $ty_name(crate::element(stringify!($name), children))
        }

        impl<ViewSeq> $ty_name<ViewSeq> {
            /// Set an attribute on this element.
            ///
            /// # Panics
            ///
            /// If the name contains characters that are not valid in an attribute name,
            /// then the `View::build`/`View::rebuild` functions will panic for this view.
            pub fn attr(
                mut self,
                name: impl Into<std::borrow::Cow<'static, str>>,
                value: impl crate::IntoAttributeValue,
            ) -> Self {
                self.0.set_attr(name, value);
                self
            }

            /// Set an attribute on this element.
            ///
            /// # Panics
            ///
            /// If the name contains characters that are not valid in an attribute name,
            /// then the `View::build`/`View::rebuild` functions will panic for this view.
            pub fn set_attr(
                &mut self,
                name: impl Into<std::borrow::Cow<'static, str>>,
                value: impl crate::IntoAttributeValue,
            ) -> &mut Self {
                self.0.set_attr(name, value);
                self
            }

            pub fn remove_attr(&mut self, name: &str) -> &mut Self {
                self.0.remove_attr(name);
                self
            }

            pub fn after_update(mut self, after_update: impl Fn(&$web_sys_ty) + 'static) -> Self {
                self.0 = self.0.after_update(after_update);
                self
            }
        }

        impl<ViewSeq> crate::view::ViewMarker for $ty_name<ViewSeq> {}

        impl<T_, A_, ViewSeq> crate::view::View<T_, A_> for $ty_name<ViewSeq>
        where
            ViewSeq: crate::view::ViewSequence<T_, A_>,
        {
            type State = crate::ElementState<ViewSeq::State>;
            type Element = $web_sys_ty;

            fn build(
                &self,
                cx: &mut crate::context::Cx,
            ) -> (xilem_core::Id, Self::State, Self::Element) {
                self.0.build(cx)
            }

            fn rebuild(
                &self,
                cx: &mut crate::context::Cx,
                prev: &Self,
                id: &mut xilem_core::Id,
                state: &mut Self::State,
                element: &mut Self::Element,
            ) -> crate::ChangeFlags {
                self.0.rebuild(cx, &prev.0, id, state, element)
            }

            fn message(
                &self,
                id_path: &[xilem_core::Id],
                state: &mut Self::State,
                message: Box<dyn std::any::Any>,
                app_state: &mut T_,
            ) -> xilem_core::MessageResult<A_> {
                self.0.message(id_path, state, message, app_state)
            }
        }
    };
}

// void elements (those without children) are `area`, `base`, `br`, `col`,
// `embed`, `hr`, `img`, `input`, `link`, `meta`, `source`, `track`, `wbr`
elements!(
    // the order is copied from
    // https://developer.mozilla.org/en-US/docs/Web/HTML/Element
    // DOM interfaces copied from https://html.spec.whatwg.org/multipage/grouping-content.html and friends

    // content sectioning
    // (Address, address, web_sys::HtmlElement),
    // (Article, article, web_sys::HtmlElement),
    // (Aside, aside, web_sys::HtmlElement),
    // (Footer, footer, web_sys::HtmlElement),
    // (Header, header, web_sys::HtmlElement),
    // (H1, h1, web_sys::HtmlHeadingElement),
    // (H2, h2, web_sys::HtmlHeadingElement),
    // (H3, h3, web_sys::HtmlHeadingElement),
    // (H4, h4, web_sys::HtmlHeadingElement),
    // (H5, h5, web_sys::HtmlHeadingElement),
    // (H6, h6, web_sys::HtmlHeadingElement),
    // (Hgroup, hgroup, web_sys::HtmlElement),
    // (Main, main, web_sys::HtmlElement),
    // (Nav, nav, web_sys::HtmlElement),
    // (Section, section, web_sys::HtmlElement),
    // text content
    (Blockquote, blockquote, web_sys::HtmlQuoteElement),
    // (Dd, dd, web_sys::HtmlElement),
    // (Div, div, web_sys::HtmlDivElement),
    (Dl, dl, web_sys::HtmlDListElement),
    // (Dt, dt, web_sys::HtmlElement),
    // (Figcaption, figcaption, web_sys::HtmlElement),
    // (Figure, figure, web_sys::HtmlElement),
    (Hr, hr, web_sys::HtmlHrElement),
    (Li, li, web_sys::HtmlLiElement),
    (Menu, menu, web_sys::HtmlMenuElement),
    (Ol, ol, web_sys::HtmlOListElement),
    (P, p, web_sys::HtmlParagraphElement),
    (Pre, pre, web_sys::HtmlPreElement),
    (Ul, ul, web_sys::HtmlUListElement),
    // inline text
    (A, a, web_sys::HtmlAnchorElement),
    // (Abbr, abbr, web_sys::HtmlElement),
    // (B, b, web_sys::HtmlElement),
    // (Bdi, bdi, web_sys::HtmlElement),
    // (Bdo, bdo, web_sys::HtmlElement),
    (Br, br, web_sys::HtmlBrElement),
    // (Cite, cite, web_sys::HtmlElement),
    // (Code, code, web_sys::HtmlElement),
    (Data, data, web_sys::HtmlDataElement),
    // (Dfn, dfn, web_sys::HtmlElement),
    // (Em, em, web_sys::HtmlElement),
    // (I, i, web_sys::HtmlElement),
    // (Kbd, kbd, web_sys::HtmlElement),
    // (Mark, mark, web_sys::HtmlElement),
    (Q, q, web_sys::HtmlQuoteElement),
    // (Rp, rp, web_sys::HtmlElement),
    // (Rt, rt, web_sys::HtmlElement),
    // (Ruby, ruby, web_sys::HtmlElement),
    // (S, s, web_sys::HtmlElement),
    // (Samp, samp, web_sys::HtmlElement),
    // (Small, small, web_sys::HtmlElement),
    // (Span, span, web_sys::HtmlSpanElement),
    // (Strong, strong, web_sys::HtmlElement),
    // (Sub, sub, web_sys::HtmlElement),
    // (Sup, sup, web_sys::HtmlElement),
    (Time, time, web_sys::HtmlTimeElement),
    // (U, u, web_sys::HtmlElement),
    // (Var, var, web_sys::HtmlElement),
    // (Wbr, wbr, web_sys::HtmlElement),
    // image and multimedia
    (Area, area, web_sys::HtmlAreaElement),
    (Audio, audio, web_sys::HtmlAudioElement),
    (Img, img, web_sys::HtmlImageElement),
    (Map, map, web_sys::HtmlMapElement),
    (Track, track, web_sys::HtmlTrackElement),
    (Video, video, web_sys::HtmlVideoElement),
    // embedded content
    (Embed, embed, web_sys::HtmlEmbedElement),
    (Iframe, iframe, web_sys::HtmlIFrameElement),
    (Object, object, web_sys::HtmlObjectElement),
    (Picture, picture, web_sys::HtmlPictureElement),
    // (Portal, portal, web_sys::HtmlElement),
    (Source, source, web_sys::HtmlSourceElement),
    // SVG and MathML (TODO, svg and mathml elements)
    // (Svg, svg, web_sys::HtmlElement),
    // (Math, math, web_sys::HtmlElement),
    // scripting
    // (Canvas, canvas, web_sys::HtmlCanvasElement),
    // (Noscript, noscript, web_sys::HtmlElement),
    (Script, script, web_sys::HtmlScriptElement),
    // demarcating edits
    (Del, del, web_sys::HtmlModElement),
    (Ins, ins, web_sys::HtmlModElement),
    // tables
    (Caption, caption, web_sys::HtmlTableCaptionElement),
    (Col, col, web_sys::HtmlTableColElement),
    (Colgroup, colgroup, web_sys::HtmlTableColElement),
    (Table, table, web_sys::HtmlTableSectionElement),
    (Tbody, tbody, web_sys::HtmlTableSectionElement),
    (Td, td, web_sys::HtmlTableCellElement),
    (Tfoot, tfoot, web_sys::HtmlTableSectionElement),
    (Th, th, web_sys::HtmlTableCellElement),
    (Thead, thead, web_sys::HtmlTableSectionElement),
    (Tr, tr, web_sys::HtmlTableRowElement),
    // forms
    (Button, button, web_sys::HtmlButtonElement),
    (Datalist, datalist, web_sys::HtmlDataListElement),
    (Fieldset, fieldset, web_sys::HtmlFieldSetElement),
    (Form, form, web_sys::HtmlFormElement),
    (Input, input, web_sys::HtmlInputElement),
    (Label, label, web_sys::HtmlLabelElement),
    (Legend, legend, web_sys::HtmlLegendElement),
    (Meter, meter, web_sys::HtmlMeterElement),
    (Optgroup, optgroup, web_sys::HtmlOptGroupElement),
    (OptionElement, option, web_sys::HtmlOptionElement), // Avoid cluttering the namespace with `Option`
    (Output, output, web_sys::HtmlOutputElement),
    (Progress, progress, web_sys::HtmlProgressElement),
    (Select, select, web_sys::HtmlSelectElement),
    (Textarea, textarea, web_sys::HtmlTextAreaElement),
    // interactive elements,
    (Details, details, web_sys::HtmlDetailsElement),
    (Dialog, dialog, web_sys::HtmlDialogElement),
    // (Summary, summary, web_sys::HtmlElement),
    // web components,
    (Slot, slot, web_sys::HtmlSlotElement),
    (Template, template, web_sys::HtmlTemplateElement),
);

pub struct VecMap<K, V>(Vec<(K, V)>);

impl<K, V> Default for VecMap<K, V> {
    fn default() -> Self {
        Self(Vec::new())
    }
}

/// Basically an ordered Map (similar as BTreeMap) with a Vec as backend for very few elements
impl<K, V> VecMap<K, V> {
    pub fn get<Q: ?Sized>(&self, key: &Q) -> Option<&V>
    where
        K: Borrow<Q> + PartialEq,
        Q: PartialEq,
    {
        self.0
            .iter()
            .find_map(|(k, v)| if key.eq(k.borrow()) { Some(v) } else { None })
    }

    pub fn get_mut<Q: ?Sized>(&mut self, key: &Q) -> Option<&mut V>
    where
        K: Borrow<Q> + Ord,
        Q: Ord,
    {
        self.0
            .iter_mut()
            .find_map(|(k, v)| if key.eq((*k).borrow()) { Some(v) } else { None })
    }

    pub fn keys(&self) -> impl Iterator<Item = &K> {
        self.0.iter().map(|(name, _)| name)
    }

    pub fn iter(&self) -> impl Iterator<Item = (&K, &V)> {
        self.0.iter().map(|(k, v)| (k, v))
    }

    pub fn diff<'a>(&'a self, other: &'a Self) -> impl Iterator<Item = Diff<&'a K, &'a V>> + 'a
    where
        K: Ord,
        V: PartialEq,
    {
        DiffMapIterator {
            prev: self.iter().peekable(),
            next: other.iter().peekable(),
        }
    }

    pub fn insert(&mut self, key: K, value: V) -> Option<V>
    where
        K: Ord,
    {
        match self.0.binary_search_by_key(&&key, |(n, _)| n) {
            Ok(pos) => {
                let mut val = (key, value);
                std::mem::swap(&mut self.0[pos], &mut val);
                Some(val.1)
            }
            Err(pos) => {
                self.0.insert(pos, (key, value));
                None
            }
        }
    }
}

pub fn diff_tree_maps<'a, K: Ord, V: PartialEq>(
    prev: &'a BTreeMap<K, V>,
    next: &'a BTreeMap<K, V>,
) -> impl Iterator<Item = Diff<&'a K, &'a V>> + 'a {
    DiffMapIterator {
        prev: prev.iter().peekable(),
        next: next.iter().peekable(),
    }
}

struct DiffMapIterator<'a, K: 'a, V: 'a, I: Iterator<Item = (&'a K, &'a V)>> {
    prev: Peekable<I>,
    next: Peekable<I>,
}

impl<'a, K: Ord + 'a, V: PartialEq, I: Iterator<Item = (&'a K, &'a V)>> Iterator
    for DiffMapIterator<'a, K, V, I>
{
    type Item = Diff<&'a K, &'a V>;
    fn next(&mut self) -> Option<Self::Item> {
        loop {
            match (self.prev.peek(), self.next.peek()) {
                (Some(&(prev_k, prev_v)), Some(&(next_k, next_v))) => match prev_k.cmp(next_k) {
                    Ordering::Less => {
                        self.prev.next();
                        return Some(Diff::Remove(prev_k));
                    }
                    Ordering::Greater => {
                        self.next.next();
                        return Some(Diff::Add(next_k, next_v));
                    }
                    Ordering::Equal => {
                        self.prev.next();
                        self.next.next();
                        if prev_v != next_v {
                            return Some(Diff::Change(next_k, next_v));
                        }
                    }
                },
                (Some(&(prev_k, _)), None) => {
                    self.prev.next();
                    return Some(Diff::Remove(prev_k));
                }
                (None, Some(&(next_k, next_v))) => {
                    self.next.next();
                    return Some(Diff::Add(next_k, next_v));
                }
                (None, None) => return None,
            }
        }
    }
}

pub enum Diff<K, V> {
    Add(K, V),
    Remove(K),
    Change(K, V),
}

type CowStr = Cow<'static, str>;

// TODO in the future it's likely there's an element that doesn't implement PartialEq,
// but for now it's simpler for diffing, maybe also use some kind of serialization in that case
#[derive(PartialEq, Debug)]
pub enum AttributeValue {
    U32(u32),
    I32(i32),
    F32(f32),
    F64(f64),
    String(CowStr),
    // for classes mostly
    // TODO maybe use Vec as backend (should probably be more performant for few classes, which seems to be the average case)
    StringBTreeSet(BTreeSet<CowStr>),
}

// TODO not sure how useful an extra enum for attribute keys is (comparison is probably a little bit faster...)
// #[derive(PartialEq, Eq)]
// enum AttrKey {
//     Width,
//     Height,
//     Class,
//     Untyped(Box<Cow<'static, str>>),
// }

impl AttributeValue {
    fn as_cow(&self) -> CowStr {
        match self {
            AttributeValue::U32(n) => n.to_string().into(),
            AttributeValue::I32(n) => n.to_string().into(),
            AttributeValue::F32(n) => n.to_string().into(),
            AttributeValue::F64(n) => n.to_string().into(),
            AttributeValue::String(s) => s.clone(),
            // currently just concatenates strings with spaces in between,
            // this may change in the future (TODO separate enum tag for classes, and e.g. comma separated lists?)
            AttributeValue::StringBTreeSet(bt) => bt
                .iter()
                .fold(String::new(), |mut acc, s| {
                    if !acc.is_empty() {
                        acc += " ";
                    }
                    if !s.is_empty() {
                        acc += s;
                    }
                    acc
                })
                .into(),
        }
    }
}

impl From<u32> for AttributeValue {
    fn from(value: u32) -> Self {
        AttributeValue::U32(value)
    }
}

impl From<i32> for AttributeValue {
    fn from(value: i32) -> Self {
        AttributeValue::I32(value)
    }
}

impl From<f32> for AttributeValue {
    fn from(value: f32) -> Self {
        AttributeValue::F32(value)
    }
}

impl From<f64> for AttributeValue {
    fn from(value: f64) -> Self {
        AttributeValue::F64(value)
    }
}

impl From<String> for AttributeValue {
    fn from(value: String) -> Self {
        AttributeValue::String(value.into())
    }
}

impl From<CowStr> for AttributeValue {
    fn from(value: CowStr) -> Self {
        AttributeValue::String(value)
    }
}

impl From<&'static str> for AttributeValue {
    fn from(value: &'static str) -> Self {
        AttributeValue::String(value.into())
    }
}

type Attrs = VecMap<CowStr, AttributeValue>;

impl Attrs {
    fn insert_attr(&mut self, name: impl Into<CowStr>, value: impl Into<AttributeValue>) {
        self.insert(name.into(), value.into());
    }
}

pub trait EventHandler<T, A = (), E = ()> {
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

impl<T, A, E: 'static, F: Fn(&mut T, E) -> A> EventHandler<T, A, E> for F {
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
            crate::MessageResult::Action(self(app_state, event.event))
        } else {
            debug_warn!("downcasting event in event handler callback failed, discarding");
            crate::MessageResult::Stale(event)
        }
    }
}

struct EventListener<T, A, E, El, EH> {
    #[allow(clippy::complexity)]
    phantom: PhantomData<fn() -> (T, A, E, El)>,
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

impl<T, A, E, El, EH> EventListener<T, A, E, El, EH>
where
    E: JsCast + 'static,
    El: 'static,
    EH: EventHandler<T, A, Event<E, El>>,
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
    pub fn new<E, EL, EH>(listener: EventListener<T, A, E, EL, EH>) -> Self
    where
        T: 'static,
        A: 'static,
        E: JsCast + 'static,
        EL: 'static,
        EH: EventHandler<T, A, Event<E, EL>> + 'static,
    {
        let build: DynamicEventListenerBuildFn<T, A> = |self_, element, cx| {
            let (id, state) = self_
                .listener
                .downcast_ref::<EventListener<T, A, E, EL, EH>>()
                .unwrap()
                .build(cx, element);
            (id, Box::new(state))
        };

        let rebuild: DynamicEventListenerRebuildFn<T, A> =
            |self_, event_target, cx, prev, id, state| {
                let listener = self_
                    .listener
                    .downcast_ref::<EventListener<T, A, E, EL, EH>>()
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
                    .downcast_ref::<EventListener<T, A, E, EL, EH>>()
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
    attrs: &Attrs,
    children: &Vec<Pod>,
    event_listeners: &[DynamicEventListener<T, A>],
) -> (web_sys::HtmlElement, EventListenersState) {
    cx.with_id(id, |cx| {
        let el = cx.create_html_element(node_name);

        for (name, value) in attrs.iter() {
            el.set_attribute(name, &value.as_cow()).unwrap_throw();
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
    attrs: &Attrs,
    prev_attrs: &Attrs,
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
    for itm in prev_attrs.diff(attrs) {
        match itm {
            Diff::Add(name, value) | Diff::Change(name, value) => {
                set_attribute(element, name, &value.as_cow());
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

/// These traits should mirror the respective DOM interfaces
/// In this case https://dom.spec.whatwg.org/#interface-element
/// Or rather a curated/opinionated subset that makes sense in xilem for each of these interfaces
/// unfortunately with this (builder + generic type parameters in methods) pattern not trait-object-safe
///
/// I'm generally not sure yet, if it makes sense to do all of this via traits (there are advantages though, but it's (likely) not compilation size...)
/// (thinking about AsRef<ParentClass> and AsMut<ParentClass>, and implementing all builder methods on the concrete type, similar as with wasm-bindgen)
/// but on the other hand most of these methods should inline to the non-generic Attrs::insert_attr (or similar non-generic impls)
pub trait Element<T, A>: Node + crate::view::View<T, A> {
    // TODO rename to class (currently conflicts with `ViewExt`)
    // TODO should classes be additive? Currently they are.
    fn classes<C: IntoClass>(self, class: C) -> Self;
    fn add_class<C: IntoClass>(&mut self, class: C);
    // TODO should this be in its own trait? (it doesn't have much to do with the DOM Node interface)
    fn raw_attrs(&self) -> &Attrs;
    // TODO should this be in Node?
    fn attr<K: Into<CowStr>, V: Into<AttributeValue>>(self, key: K, value: V) -> Self;
    fn set_attr<K: Into<CowStr>, V: Into<AttributeValue>>(&mut self, key: K, value: V);

    // TODO generate all this event listener boilerplate with macros
    fn on_click<EH>(self, handler: EH) -> Self
    where
        T: 'static,
        A: 'static,
        EH: EventHandler<T, A, crate::Event<web_sys::MouseEvent, Self::Element>> + 'static;

    fn on_click_with_options<EH>(self, handler: EH, options: EventListenerOptions) -> Self
    where
        T: 'static,
        A: 'static,
        EH: EventHandler<T, A, crate::Event<web_sys::MouseEvent, Self::Element>> + 'static;

    fn on_scroll<EH>(self, handler: EH) -> Self
    where
        Self: Sized,
        T: 'static,
        A: 'static,
        EH: EventHandler<T, A, crate::Event<web_sys::Event, Self::Element>> + 'static;

    fn on_scroll_with_options<EH>(self, handler: EH, options: EventListenerOptions) -> Self
    where
        T: 'static,
        A: 'static,
        EH: EventHandler<T, A, crate::Event<web_sys::Event, Self::Element>> + 'static;

    // TODO rest of all the methods allowed on an element
}

pub trait HtmlElement<T, A>: Element<T, A> {}

pub trait HtmlDivElement<T, A>: HtmlElement<T, A> {
    // TODO "align" attr
}

pub trait HtmlSpanElement<T, A>: HtmlElement<T, A> {}

pub trait HtmlHeadingElement<T, A>: HtmlElement<T, A> {
    // TODO "align" attr
}

// not sure if an extra trait for this makes sense, but for consistency
pub trait HtmlCanvasElement<T, A>: HtmlElement<T, A> {
    fn width(self, width: u32) -> Self;
    fn set_width(&mut self, width: u32);

    fn height(self, height: u32) -> Self;
    fn set_height(&mut self, height: u32);
}

fn add_class<C: IntoClass>(attrs: &mut Attrs, class: C) {
    let mut classes = class.classes().peekable();

    if classes.peek().is_none() {
        return;
    }

    match attrs.get_mut("class") {
        Some(AttributeValue::StringBTreeSet(attr_value)) => {
            attr_value.extend(classes);
        }
        // could be useful, in case untyped values are inserted here
        Some(untyped_class) if matches!(untyped_class, AttributeValue::String(_)) => {
            let mut classes = BTreeSet::from_iter(classes);
            classes.insert(if let AttributeValue::String(s) = untyped_class {
                s.clone()
            } else {
                unreachable!()
            });
            *untyped_class = AttributeValue::StringBTreeSet(classes);
        }
        Some(other) => {
            // TODO warning
            // panic!("A static attribute 'class' should always have either the type BTreeSet<CowStr> or String")
            *other = AttributeValue::StringBTreeSet(BTreeSet::from_iter(classes));
        }
        None => {
            attrs.insert(
                "class".into(),
                AttributeValue::StringBTreeSet(BTreeSet::from_iter(classes)),
            );
        }
    };
}

macro_rules! impl_simple_attr {
    ($name:ident, $setter_name: ident, $ty: ty, $el: ident) => {
        #[inline(always)]
        fn $name(mut self, $name: $ty) -> $el<T, A, VS> {
            self.attrs.insert_attr(stringify!($name), $name);
            self
        }

        #[inline(always)]
        fn $setter_name(&mut self, $name: $ty) {
            self.attrs.insert_attr(stringify!($name), $name);
        }
    };
}

// Since these methods are used for all HTML elements,
// it might make sense to add an extra inner impl function if possible
// (see below at `simple_attr_impl` for an example) to avoid big compilation code size
macro_rules! impl_element {
    ($ty_name:ident, $name: ident, $concrete_dom_interface: ident) => {
        impl<T, A, VS> Element<T, A> for $ty_name<T, A, VS>
        where
            VS: crate::view::ViewSequence<T, A>,
        {
            fn classes<C: IntoClass>(mut self, class: C) -> Self {
                add_class(&mut self.attrs, class);
                self
            }

            fn add_class<C: IntoClass>(&mut self, class: C) {
                add_class(&mut self.attrs, class);
            }

            fn raw_attrs(&self) -> &Attrs {
                &self.attrs
            }

            fn attr<K: Into<CowStr>, V: Into<AttributeValue>>(
                mut self,
                key: K,
                value: V,
            ) -> $ty_name<T, A, VS> {
                self.attrs.insert_attr(key, value);
                self
            }

            fn set_attr<K: Into<CowStr>, V: Into<AttributeValue>>(&mut self, key: K, value: V) {
                self.attrs.insert_attr(key, value);
            }

            fn on_click<EH>(self, handler: EH) -> $ty_name<T, A, VS>
            where
                T: 'static,
                A: 'static,
                EH: EventHandler<
                        T,
                        A,
                        crate::Event<web_sys::MouseEvent, web_sys::$concrete_dom_interface>,
                    > + 'static, // V::Element, but this results in better docs
            {
                self.on_click_with_options(handler, EventListenerOptions::default())
            }

            fn on_click_with_options<EH>(
                mut self,
                handler: EH,
                options: EventListenerOptions,
            ) -> $ty_name<T, A, VS>
            where
                T: 'static,
                A: 'static,
                EH: EventHandler<
                        T,
                        A,
                        crate::Event<web_sys::MouseEvent, web_sys::$concrete_dom_interface>,
                    > + 'static, // V::Element, but this results in better docs
            {
                let listener = EventListener::new("click", handler, options);
                self.event_listeners
                    .push(DynamicEventListener::new(listener));
                self
            }

            fn on_scroll<EH>(self, handler: EH) -> $ty_name<T, A, VS>
            where
                T: 'static,
                A: 'static,
                EH: EventHandler<
                        T,
                        A,
                        crate::Event<web_sys::Event, web_sys::$concrete_dom_interface>,
                    > + 'static, // V::Element, but this results in better docs
            {
                self.on_scroll_with_options(handler, EventListenerOptions::default())
            }

            fn on_scroll_with_options<EH>(
                mut self,
                handler: EH,
                options: EventListenerOptions,
            ) -> $ty_name<T, A, VS>
            where
                T: 'static,
                A: 'static,
                EH: EventHandler<
                        T,
                        A,
                        crate::Event<web_sys::Event, web_sys::$concrete_dom_interface>,
                    > + 'static, // V::Element, but this results in better docs
            {
                let listener = EventListener::new("scroll", handler, options);
                self.event_listeners
                    .push(DynamicEventListener::new(listener));
                self
            }
        }
    };
}

macro_rules! generate_dom_interface_impl {
    ($ty_name:ident, $name:ident, $dom_interface:ident) => {
        generate_dom_interface_impl!($ty_name, $name, $dom_interface, {});
    };
    ($ty_name:ident, $name:ident, $dom_interface:ident, $body: tt) => {
        impl<T, A, VS> $dom_interface<T, A> for $ty_name<T, A, VS>
        where
            VS: crate::view::ViewSequence<T, A>,
        $body
    };
}

macro_rules! impl_html_dom_interface {
    ($ty_name: ident, $name: ident, Node) => {
        impl<T, A, VS> Node for $ty_name<T, A, VS> {
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
    ($ty_name: ident, $name: ident, $concrete_dom_interface: ident, HtmlDivElement) => {
        impl_html_dom_interface!($ty_name, $name, $concrete_dom_interface, HtmlElement);
        generate_dom_interface_impl!($ty_name, $name, HtmlDivElement);
    };
    ($ty_name: ident, $name: ident, $concrete_dom_interface: ident, HtmlSpanElement) => {
        impl_html_dom_interface!($ty_name, $name, $concrete_dom_interface, HtmlElement);
        generate_dom_interface_impl!($ty_name, $name, HtmlSpanElement);
    };
    ($ty_name: ident, $name: ident, $concrete_dom_interface: ident, HtmlHeadingElement) => {
        impl_html_dom_interface!($ty_name, $name, $concrete_dom_interface, HtmlElement);
        generate_dom_interface_impl!($ty_name, $name, HtmlHeadingElement);
    };
    ($ty_name: ident, $name: ident, $concrete_dom_interface: ident, HtmlCanvasElement) => {
        impl_html_dom_interface!($ty_name, $name, $concrete_dom_interface, HtmlElement);
        generate_dom_interface_impl!($ty_name, $name, HtmlCanvasElement, {
            impl_simple_attr!(width, set_width, u32, $ty_name);
            impl_simple_attr!(height, set_height, u32, $ty_name);
        });
    };
}

macro_rules! define_html_elements {
    ($(($ty_name:ident, $name:ident, $dom_interface:ident),)*) => {
        $(
        // TODO not sure how much it helps reducing the code size,
        // but the two attributes could be extracted into its own type, and the actual element type is just a single tuple struct wrapping this type,
        pub struct $ty_name<T, A, VS> {
            pub(crate) attrs: Attrs,
            event_listeners: Vec<DynamicEventListener<T, A>>,
            children: VS,
            phantom: std::marker::PhantomData<fn() -> (T, A)>,
        }

        impl<T, A, VS> crate::view::ViewMarker for $ty_name<T, A, VS> {}

        impl<T, A, VS> crate::view::View<T, A> for $ty_name<T, A, VS>
        where
            VS: crate::view::ViewSequence<T, A>,
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
                app_state: &mut T,
            ) -> xilem_core::MessageResult<A> {
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
        pub fn $name<T, A, VS>(children: VS) -> $ty_name<T, A, VS>
        where
            VS: crate::view::ViewSequence<T, A>,
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
    // (Blockquote, blockquote, HtmlQuoteElement),
    (Dd, dd, HtmlElement),
    (Div, div, HtmlDivElement),
    // (Dl, dl, HtmlDListElement),
    (Dt, dt, HtmlElement),
    (Figcaption, figcaption, HtmlElement),
    (Figure, figure, HtmlElement),
    // (Hr, hr, HtmlHrElement),
    // (Li, li, HtmlLiElement),
    // (Menu, menu, HtmlMenuElement),
    // (Ol, ol, HtmlOListElement),
    // (P, p, HtmlParagraphElement),
    // (Pre, pre, HtmlPreElement),
    // (Ul, ul, HtmlUListElement),
    // inline text
    // (A, a, HtmlAnchorElement),
    (Abbr, abbr, HtmlElement),
    (B, b, HtmlElement),
    (Bdi, bdi, HtmlElement),
    (Bdo, bdo, HtmlElement),
    // (Br, br, HtmlBrElement),
    (Cite, cite, HtmlElement),
    (Code, code, HtmlElement),
    // (Data, data, HtmlDataElement),
    (Dfn, dfn, HtmlElement),
    (Em, em, HtmlElement),
    (I, i, HtmlElement),
    (Kbd, kbd, HtmlElement),
    (Mark, mark, HtmlElement),
    // (Q, q, HtmlQuoteElement),
    (Rp, rp, HtmlElement),
    (Rt, rt, HtmlElement),
    (Ruby, ruby, HtmlElement),
    (S, s, HtmlElement),
    (Samp, samp, HtmlElement),
    (Small, small, HtmlElement),
    (Span, span, HtmlSpanElement), // TODO HtmlSpanElement
    (Strong, strong, HtmlElement),
    (Sub, sub, HtmlElement),
    (Sup, sup, HtmlElement),
    // (Time, time, HtmlTimeElement),
    (U, u, HtmlElement),
    (Var, var, HtmlElement),
    (Wbr, wbr, HtmlElement),
    // image and multimedia
    // (Area, area, HtmlAreaElement),
    // (Audio, audio, HtmlAudioElement),
    // (Img, img, HtmlImageElement),
    // (Map, map, HtmlMapElement),
    // (Track, track, HtmlTrackElement),
    // (Video, video, HtmlVideoElement),
    // embedded content
    // (Embed, embed, HtmlEmbedElement),
    // (Iframe, iframe, HtmlIFrameElement),
    // (Object, object, HtmlObjectElement),
    // (Picture, picture, HtmlPictureElement),
    (Portal, portal, HtmlElement),
    // (Source, source, HtmlSourceElement),
    // SVG and MathML (TODO, svg and mathml elements)
    (Svg, svg, HtmlElement),
    (Math, math, HtmlElement),
    // scripting
    (Canvas, canvas, HtmlCanvasElement),
    (Noscript, noscript, HtmlElement),
    // (Script, script, HtmlScriptElement),
    // demarcating edits
    // (Del, del, HtmlModElement),
    // (Ins, ins, HtmlModElement),
    // tables
    // (Caption, caption, HtmlTableCaptionElement),
    // (Col, col, HtmlTableColElement),
    // (Colgroup, colgroup, HtmlTableColElement),
    // (Table, table, HtmlTableSectionElement),
    // (Tbody, tbody, HtmlTableSectionElement),
    // (Td, td, HtmlTableCellElement),
    // (Tfoot, tfoot, HtmlTableSectionElement),
    // (Th, th, HtmlTableCellElement),
    // (Thead, thead, HtmlTableSectionElement),
    // (Tr, tr, HtmlTableRowElement),
    // forms
    // (Button, button, HtmlButtonElement),
    // (Datalist, datalist, HtmlDataListElement),
    // (Fieldset, fieldset, HtmlFieldSetElement),
    // (Form, form, HtmlFormElement),
    // (Input, input, HtmlInputElement),
    // (Label, label, HtmlLabelElement),
    // (Legend, legend, HtmlLegendElement),
    // (Meter, meter, HtmlMeterElement),
    // (Optgroup, optgroup, HtmlOptGroupElement),
    // (OptionElement, option, web_sys::HtmlOptionElement), // Avoid cluttering the namespace with `Option`
    // (Output, output, HtmlOutputElement),
    // (Progress, progress, HtmlProgressElement),
    // (Select, select, HtmlSelectElement),
    // (Textarea, textarea, HtmlTextAreaElement),
    // interactive elements,
    // (Details, details, HtmlDetailsElement),
    // (Dialog, dialog, HtmlDialogElement),
    (Summary, summary, HtmlElement),
    // web components,
    // (Slot, slot, HtmlSlotElement),
    // (Template, template, HtmlTemplateElement),
);

// A few experiments for more flexible attributes (el.class<C: IntoClass>(class: C))
pub trait IntoClass {
    type ClassIter: Iterator<Item = CowStr>;
    fn classes(self) -> Self::ClassIter;
}

impl IntoClass for &'static str {
    type ClassIter = std::option::IntoIter<CowStr>;
    fn classes(self) -> Self::ClassIter {
        Some(self.into()).into_iter()
    }
}

impl IntoClass for String {
    type ClassIter = std::option::IntoIter<CowStr>;
    fn classes(self) -> Self::ClassIter {
        Some(self.into()).into_iter()
    }
}

impl IntoClass for CowStr {
    type ClassIter = std::option::IntoIter<CowStr>;
    fn classes(self) -> Self::ClassIter {
        Some(self).into_iter()
    }
}

impl<T: IntoClass, const N: usize> IntoClass for [T; N] {
    // we really need impl
    type ClassIter =
        std::iter::FlatMap<std::array::IntoIter<T, N>, T::ClassIter, fn(T) -> T::ClassIter>;
    fn classes(self) -> Self::ClassIter {
        self.into_iter().flat_map(IntoClass::classes)
    }
}

impl<T: IntoClass> IntoClass for Vec<T> {
    type ClassIter = std::iter::FlatMap<std::vec::IntoIter<T>, T::ClassIter, fn(T) -> T::ClassIter>;
    fn classes(self) -> Self::ClassIter {
        self.into_iter().flat_map(IntoClass::classes)
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
    fn classes(self) -> Self::ClassIter {
        self.0.classes().chain(self.1.classes())
    }
}
