//! Types that wrap [`Element`][super::Element] and represent specific element types.
//!

use std::{
    any::Any,
    collections::{BTreeMap, BTreeSet},
};

use wasm_bindgen::JsCast;

use crate::Pod;
use wasm_bindgen::UnwrapThrowExt;

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
    (Option, option, web_sys::HtmlOptionElement),
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

// TODO think about using serialized values instead of Box<dyn Any> for smaller compilation size (but likely worse performance)
// TODO consider enum for common attribute types and Box<dyn Any> as fallback for more exotic cases
type Attrs = BTreeMap<&'static str, Box<dyn Any>>;

const UNTYPED_ATTRS: &str = "____untyped_attrs____";

/// returns all attribute keys including untyped attributes
fn attr_keys(attrs: &Attrs) -> impl Iterator<Item = &str> {
    attrs.keys().copied().filter(|a| *a != UNTYPED_ATTRS).chain(
        attrs
            .get(UNTYPED_ATTRS)
            .map(|untyped_attrs| {
                untyped_attrs
                    .downcast_ref::<BTreeMap<String, String>>()
                    .unwrap()
                    .keys()
                    .map(|k| k.as_str())
            })
            .into_iter()
            .flatten(),
    )
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
pub trait Element: Node {
    // TODO rename to class (currently conflicts with `ViewExt`)
    fn classes<T: IntoClass>(self, class: T) -> Self;
    // TODO rename to class (currently conflicts with `ViewExt`)
    fn add_classes<T: IntoClass>(&mut self, class: T);
    // TODO should this be in its own trait? (it doesn't have much to do with the DOM Node interface)
    fn raw_attrs(&self) -> &Attrs;
    // TODO should this be in Node?
    fn attr<K: Into<String>, V: Into<String>>(self, key: K, value: V) -> Self;
    fn set_attr<K: Into<String>, V: Into<String>>(&mut self, key: K, value: V);
}

pub trait HtmlElement: Element {}

pub trait HtmlDivElement: HtmlElement {
    // TODO "align" attr
}

pub trait HtmlSpanElement: HtmlElement {}

pub trait HtmlHeadingElement: HtmlElement {
    // TODO "align" attr
}

// not sure if an extra trait for this makes sense, but for consistency
pub trait HtmlCanvasElement: HtmlElement + Sized {
    fn width(self, width: u32) -> Self;
    fn set_width(&mut self, width: u32);

    fn height(self, height: u32) -> Self;
    fn set_height(&mut self, height: u32);
}

fn simple_attr_impl<T: 'static>(
    attrs: &mut BTreeMap<&'static str, Box<dyn Any>>,
    key: &'static str,
    value: T,
) {
    match attrs.entry(key) {
        std::collections::btree_map::Entry::Vacant(entry) => {
            entry.insert(Box::new(value));
        }
        std::collections::btree_map::Entry::Occupied(class_attr) => {
            *class_attr.into_mut().downcast_mut::<T>().unwrap() = value;
        }
    };
}

macro_rules! impl_simple_attr {
    ($name:ident, $setter_name: ident, $ty: ty, $el: ident) => {
        #[inline(always)]
        fn $name(mut self, $name: $ty) -> $el<T, A, VS> {
            simple_attr_impl(&mut self.attrs, stringify!($name), $name);
            self
        }

        #[inline(always)]
        fn $setter_name(&mut self, $name: $ty) {
            simple_attr_impl(&mut self.attrs, stringify!($name), $name);
        }
    };
}

macro_rules! impl_node {
    ($ty_name:ident, $name: ident) => {
        impl<T, A, VS> Node for $ty_name<T, A, VS> {
            fn node_name(&self) -> &str {
                stringify!($name)
            }
        }
    };
}

fn add_class<C: IntoClass>(attrs: &mut Attrs, class: C) {
    match attrs.entry("class") {
        std::collections::btree_map::Entry::Vacant(entry) => {
            let classes = BTreeSet::from_iter(class.classes());
            if !classes.is_empty() {
                entry.insert(Box::new(classes));
            }
        }
        std::collections::btree_map::Entry::Occupied(class_attr) => {
            class_attr
                .into_mut()
                .downcast_mut::<BTreeSet<String>>()
                .unwrap()
                .extend(class.classes());
        }
    };
}

// TODO currently untyped attributes are overwritten by typed ones,
// if they're defined, maybe (some) can be merged nicely
fn add_untyped_attr(attrs: &mut Attrs, key: String, value: String) {
    match attrs.entry(UNTYPED_ATTRS) {
        std::collections::btree_map::Entry::Vacant(entry) => {
            entry.insert(Box::new(BTreeMap::from([(key, value)])));
        }
        std::collections::btree_map::Entry::Occupied(attrs) => {
            attrs
                .into_mut()
                .downcast_mut::<BTreeMap<String, String>>()
                .unwrap()
                .insert(key, value);
        }
    };
}

// Since these methods are used for all HTML elements,
// it might make sense to add an extra inner impl function if possible
// (see below at `simple_attr_impl` for an example) to avoid big compilation code size
macro_rules! impl_element {
    ($ty_name:ident, $name: ident) => {
        impl_node!($ty_name, $name);

        impl<T, A, VS> Element for $ty_name<T, A, VS> {
            fn classes<C: IntoClass>(mut self, class: C) -> $ty_name<T, A, VS> {
                add_class(&mut self.attrs, class);
                self
            }

            fn add_classes<C: IntoClass>(&mut self, class: C) {
                add_class(&mut self.attrs, class);
            }

            fn raw_attrs(&self) -> &Attrs {
                &self.attrs
            }

            fn set_attr<K: Into<String>, V: Into<String>>(&mut self, key: K, value: V) {
                add_untyped_attr(&mut self.attrs, key.into(), value.into());
            }

            fn attr<K: Into<String>, V: Into<String>>(
                mut self,
                key: K,
                value: V,
            ) -> $ty_name<T, A, VS> {
                add_untyped_attr(&mut self.attrs, key.into(), value.into());
                self
            }
        }
    };
}

macro_rules! impl_html_element {
    ($ty_name:ident, $name: ident) => {
        impl_element!($ty_name, $name);

        impl<T, A, VS> HtmlElement for $ty_name<T, A, VS> {}
    };
}

macro_rules! generate_html_element_derivation {
    ($ty_name:ident, $name:ident, $dom_interface:ident) => {
        generate_html_element_derivation!($ty_name, $name, $dom_interface, {});
    };
    ($ty_name:ident, $name:ident, $dom_interface:ident, $body: tt) => {
        impl_html_element!($ty_name, $name);

        impl<T, A, VS> $dom_interface for $ty_name<T, A, VS> $body
    };
}

macro_rules! impl_html_dom_interface {
    ($ty_name: ident, $name: ident, Element) => {
        impl_element!($ty_name, $name);
    };
    ($ty_name: ident, $name: ident, HtmlElement) => {
        impl_html_element!($ty_name, $name);
    };
    ($ty_name: ident, $name: ident, HtmlDivElement) => {
        generate_html_element_derivation!($ty_name, $name, HtmlDivElement);
    };
    ($ty_name: ident, $name: ident, HtmlSpanElement) => {
        generate_html_element_derivation!($ty_name, $name, HtmlSpanElement);
    };
    ($ty_name: ident, $name: ident, HtmlHeadingElement) => {
        generate_html_element_derivation!($ty_name, $name, HtmlHeadingElement);
    };
    ($ty_name: ident, $name: ident, HtmlCanvasElement) => {
        generate_html_element_derivation!($ty_name, $name, HtmlCanvasElement, {
            impl_simple_attr!(width, set_width, u32, $ty_name);
            impl_simple_attr!(height, set_height, u32, $ty_name);
        });
    };
}

fn serialize_attr<'a, T, F>(
    attrs: &Attrs,
    key: &'a str,
    attr_map: &mut BTreeMap<&'a str, String>,
    serialize_fn: F,
) where
    T: PartialEq + 'static,
    F: Fn(&T) -> String,
{
    if let Some(attr) = attrs.get(key) {
        let attr = attr.downcast_ref::<T>().unwrap();
        let serialized_attr = serialize_fn(attr);
        attr_map.insert(key, serialized_attr);
    }
}

fn serialize_classes(classes: &BTreeSet<String>) -> String {
    classes.iter().fold(String::new(), |mut acc, s| {
        if !acc.is_empty() {
            acc += " ";
        }
        if !s.is_empty() {
            acc += s;
        }
        acc
    })
}

fn impl_serialize_element_attrs<'a>(attrs: &'a Attrs, attr_map: &mut BTreeMap<&'a str, String>) {
    if let Some(untyped_attrs) = attrs
        .get(UNTYPED_ATTRS)
        .and_then(|attrs| attrs.downcast_ref::<BTreeMap<String, String>>())
    {
        attr_map.extend(untyped_attrs.iter().map(|(k, v)| (k.as_str(), v.clone())));
    }
    serialize_attr(attrs, "class", attr_map, serialize_classes);
}

fn serialize_element_attrs<'a, T: Element>(
    element: &'a T,
    attr_map: &mut BTreeMap<&'a str, String>,
) {
    impl_serialize_element_attrs(element.raw_attrs(), attr_map)
}

fn serialize_html_canvas_element_attrs<'a, T: HtmlCanvasElement>(
    element: &'a T,
    attr_map: &mut BTreeMap<&'a str, String>,
) {
    serialize_element_attrs(element, attr_map);
    serialize_attr(element.raw_attrs(), "width", attr_map, u32::to_string);
    serialize_attr(element.raw_attrs(), "height", attr_map, u32::to_string);
}

macro_rules! serialize_attrs {
    ($element: expr, $attr_map: expr, Element) => {
        serialize_element_attrs($element, $attr_map);
    };
    ($element: expr, $attr_map: expr, HtmlElement) => {
        serialize_element_attrs($element, $attr_map);
    };
    ($element: expr, $attr_map: expr, HtmlDivElement) => {
        serialize_element_attrs($element, $attr_map);
    };
    ($element: expr, $attr_map: expr, HtmlSpanElement) => {
        serialize_element_attrs($element, $attr_map);
    };
    ($element: expr, $attr_map: expr, HtmlHeadingElement) => {
        serialize_element_attrs($element, $attr_map);
    };
    ($element: expr, $attr_map: expr, HtmlCanvasElement) => {
        serialize_html_canvas_element_attrs($element, $attr_map);
    };
}

fn simple_diff_attr<'a, T, F>(
    old_attrs: &Attrs,
    new_attrs: &'a Attrs,
    key: &'a str,
    new_serialized_attrs: &mut BTreeMap<&'a str, String>,
    serialize_fn: F,
) where
    T: PartialEq + 'static,
    F: Fn(&T) -> String,
{
    match (old_attrs.get(key), new_attrs.get(key)) {
        (_, None) => (),
        (None, Some(new)) => {
            let new = new.downcast_ref::<T>().unwrap();
            let serialized_attr = serialize_fn(new);
            new_serialized_attrs.insert(key, serialized_attr);
        }
        (Some(old), Some(new)) => {
            let new = new.downcast_ref::<T>().unwrap();
            if old.downcast_ref::<T>().unwrap() != new {
                let serialized_attr = serialize_fn(new);
                new_serialized_attrs.insert(key, serialized_attr);
            }
        }
    };
}

fn impl_element_diff<'a>(
    old: &Attrs,
    new: &'a Attrs,
    new_serialized_attrs: &mut BTreeMap<&'a str, String>,
) {
    match (old.get(UNTYPED_ATTRS), new.get(UNTYPED_ATTRS)) {
        (_, None) => (),
        (None, Some(new)) => {
            let new = new.downcast_ref::<BTreeMap<String, String>>().unwrap();
            new_serialized_attrs.extend(new.iter().map(|(k, v)| (k.as_str(), v.clone())));
        }
        (Some(old), Some(new)) => {
            let new = new.downcast_ref::<BTreeMap<String, String>>().unwrap();
            let old = old.downcast_ref::<BTreeMap<String, String>>().unwrap();
            // TODO there are likely faster ways to do this...
            let new_untyped_attrs = new.iter().filter_map(|(new_name, new_value)| {
                if let Some(old_value) = old.get(new_name) {
                    if new_value == old_value {
                        None
                    } else {
                        Some((new_name.as_str(), new_value.clone()))
                    }
                } else {
                    Some((new_name.as_str(), new_value.clone()))
                }
            });
            new_serialized_attrs.extend(new_untyped_attrs);
        }
    };
    simple_diff_attr(old, new, "class", new_serialized_attrs, serialize_classes);
}

/// returns whether attributes belonging to the Element interface are different (currently just `class`)
// TODO include custom "untyped" attributes
#[allow(unused)]
#[inline(always)]
fn element_diff<'a, T: Element>(old: &T, new: &'a T, diff_map: &mut BTreeMap<&'a str, String>) {
    impl_element_diff(old.raw_attrs(), new.raw_attrs(), diff_map);
}

#[allow(unused)]
#[inline(always)]
pub fn html_element_diff<'a, T: HtmlElement>(
    old: &T,
    new: &'a T,
    new_attrs: &mut BTreeMap<&'a str, String>,
) {
    element_diff(old, new, new_attrs)
}

/// returns whether attributes belonging to the HTMLCanvasElement interface are different
#[allow(unused)]
#[inline(always)]
pub fn html_canvas_element_diff<'a, T: HtmlCanvasElement>(
    old: &T,
    new: &'a T,
    new_serialized_attrs: &mut BTreeMap<&'a str, String>,
) {
    // html_element_diff(old, new, new_serialized_attrs);
    let old = old.raw_attrs();
    let new = new.raw_attrs();
    simple_diff_attr(old, new, "width", new_serialized_attrs, u32::to_string);
    simple_diff_attr(old, new, "height", new_serialized_attrs, u32::to_string);
}

macro_rules! impl_rebuild_diff {
    ($old: expr, $new: expr, $diff_map: expr, Element) => {
        element_diff($old, $new, $diff_map);
    };
    ($old: expr, $new: expr, $diff_map: expr, HtmlElement) => {
        html_element_diff($old, $new, $diff_map);
    };
    ($old: expr, $new: expr, $diff_map: expr, HtmlDivElement) => {
        html_element_diff($old, $new, $diff_map);
    };
    ($old: expr, $new: expr, $diff_map: expr, HtmlSpanElement) => {
        html_element_diff($old, $new, $diff_map);
    };
    ($old: expr, $new: expr, $diff_map: expr, HtmlHeadingElement) => {
        html_element_diff($old, $new, $diff_map);
    };
    ($old: expr, $new: expr, $diff_map: expr, HtmlCanvasElement) => {
        html_canvas_element_diff($old, $new, $diff_map);
    };
}

fn create_element(
    name: &str,
    id: xilem_core::Id,
    attributes: &mut BTreeMap<&str, String>,
    children: &Vec<Pod>,
    cx: &crate::context::Cx,
) -> web_sys::HtmlElement {
    let el = cx.create_html_element(name);

    for (name, value) in attributes {
        el.set_attribute(name, value).unwrap_throw();
    }
    // Set the id used internally to the `data-debugid` attribute.
    // This allows the user to see if an element has been re-created or only altered.
    #[cfg(debug_assertions)]
    el.set_attribute("data-debugid", &id.to_raw().to_string())
        .unwrap_throw();

    for child in children {
        el.append_child(child.0.as_node_ref()).unwrap_throw();
    }
    el
}

fn update_element(
    element: &web_sys::Element,
    prev_attrs: &Attrs,
    new_attrs: &Attrs,
    changed_attrs: &BTreeMap<&str, String>,
    children_updated: bool,
    children: &Vec<Pod>,
) {
    // TODO there's likely a faster way...
    let removed_attrs = attr_keys(prev_attrs).filter(|prev_attr_name| {
        !attr_keys(new_attrs).any(|new_attr_name| new_attr_name == *prev_attr_name)
            && !changed_attrs.contains_key(*prev_attr_name)
    });

    for attr in removed_attrs {
        remove_attribute(element, attr);
    }

    for (attr, value) in changed_attrs {
        set_attribute(element, attr, value);
    }

    if children_updated {
        // This is crude and will result in more DOM traffic than needed.
        // The right thing to do is diff the new state of the children id
        // vector against the old, and derive DOM mutations from that.
        while let Some(child) = element.first_child() {
            element.remove_child(&child).unwrap_throw();
        }
        for child in children {
            element.append_child(child.0.as_node_ref()).unwrap_throw();
        }
    }
}

macro_rules! define_html_elements {
    ($(($ty_name:ident, $name:ident, $dom_interface:ident),)*) => {
        $(
        // TODO not sure how much it helps reducing the code size,
        // but the two attributes could be extracted into its own type, and the actual element type is just a single tuple struct wrapping this type,
        pub struct $ty_name<T, A, VS> {
            pub(crate) attrs: Attrs,
            #[allow(unused)]
            children: VS,
            phantom: std::marker::PhantomData<fn() -> (T, A)>,
        }

        impl<T, A, VS> crate::view::ViewMarker for $ty_name<T, A, VS> {}

        impl<T, A, VS> crate::view::View<T, A> for $ty_name<T, A, VS>
        where
            VS: crate::view::ViewSequence<T, A>,
        {
            type State = crate::ElementState<VS::State>;
            type Element = web_sys::$dom_interface;

            fn build(
                &self,
                cx: &mut crate::context::Cx,
            ) -> (xilem_core::Id, Self::State, Self::Element) {
                // TODO remove
                web_sys::console::log_1(&format!("new element built: {}", self.node_name()).into());
                use wasm_bindgen::UnwrapThrowExt;

                let mut child_elements = vec![];
                let (id, child_states) = cx.with_new_id(|cx| self.children.build(cx, &mut child_elements));
                let mut attributes: BTreeMap<&'_ str, String> = BTreeMap::new();
                serialize_attrs!(self, &mut attributes, $dom_interface);
                let el = create_element(self.node_name(), id, &mut attributes, &child_elements, cx)
                    .dyn_into()
                    .unwrap_throw();

                let state = crate::ElementState {
                    child_states,
                    child_elements,
                    scratch: vec![],
                };
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
                use crate::ChangeFlags;
                debug_assert!(prev.node_name() == self.node_name());

                let mut new_serialized_attrs = BTreeMap::new();

                impl_rebuild_diff!(prev, self, &mut new_serialized_attrs, $dom_interface);

                // TODO remove
                web_sys::console::log_1(&format!("updated element: {}, diff_attrs: {:?}", self.node_name(), new_serialized_attrs).into());

                let mut changed = if new_serialized_attrs.is_empty() {
                    ChangeFlags::empty()
                } else {
                    ChangeFlags::OTHER_CHANGE
                };

                // update children
                let mut splice = xilem_core::VecSplice::new(&mut state.child_elements, &mut state.scratch);
                changed |= cx.with_id(*id, |cx| {
                    self.children
                        .rebuild(cx, &prev.children, &mut state.child_states, &mut splice)
                });

                let children_updated = changed.contains(ChangeFlags::STRUCTURE);
                changed.remove(ChangeFlags::STRUCTURE);

                let element = element.dyn_ref().unwrap_throw();

                update_element(
                    element,
                    prev.raw_attrs(),
                    self.raw_attrs(),
                    &new_serialized_attrs,
                    children_updated,
                    &state.child_elements
                );

                changed
            }

            fn message(
                &self,
                id_path: &[xilem_core::Id],
                state: &mut Self::State,
                message: Box<dyn std::any::Any>,
                app_state: &mut T,
            ) -> xilem_core::MessageResult<A> {
                self.children
                    .message(id_path, &mut state.child_states, message, app_state)
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
                phantom: std::marker::PhantomData
            }
        }

        impl_html_dom_interface!($ty_name, $name, $dom_interface);
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
    // (Option, option, HtmlOptionElement),
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
    type ClassIter: Iterator<Item = String>;
    fn classes(self) -> Self::ClassIter;
}

impl IntoClass for &str {
    type ClassIter = std::option::IntoIter<String>;
    fn classes(self) -> Self::ClassIter {
        Some(self.into()).into_iter()
    }
}

impl IntoClass for String {
    type ClassIter = std::option::IntoIter<String>;
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
