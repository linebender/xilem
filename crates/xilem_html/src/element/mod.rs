//! The HTML element view and associated types/functions.
//!
//! If you are writing your own views, we recommend adding
//! `use xilem_html::elements as el` or similar to the top of your file.
use crate::{
    context::{ChangeFlags, Cx},
    view::{DomElement, DomNode, Pod, View, ViewMarker, ViewSequence},
};

use std::{borrow::Cow, cmp::Ordering, collections::BTreeMap, fmt, marker::PhantomData};
use wasm_bindgen::{JsCast, UnwrapThrowExt};
use xilem_core::{Id, MessageResult, VecSplice};

#[cfg(feature = "typed")]
pub mod elements;

/// A view representing a HTML element.
///
/// If the element has no chilcdren, use the unit type (e.g. `let view = element("div", ())`).
pub struct Element<El, Children = ()> {
    name: Cow<'static, str>,
    attributes: BTreeMap<Cow<'static, str>, Cow<'static, str>>,
    children: Children,
    ty: PhantomData<El>,
}

pub trait ElementTag {
    type WebSysElement: JsCast + DomElement;

    fn name() -> &'static str;
}

impl<E, ViewSeq> Element<E, ViewSeq> {
    pub fn debug_as_el(&self) -> impl fmt::Debug + '_ {
        struct DebugFmt<'a, E, VS>(&'a Element<E, VS>);
        impl<'a, E, VS> fmt::Debug for DebugFmt<'a, E, VS> {
            fn fmt(&self, f: &mut fmt::Formatter) -> fmt::Result {
                write!(f, "<{}", self.0.name)?;
                for (name, value) in &self.0.attributes {
                    write!(f, " {name}=\"{value}\"")?;
                }
                write!(f, ">")
            }
        }
        DebugFmt(self)
    }
}

/// The state associated with a HTML element `View`.
///
/// Stores handles to the child elements and any child state.
pub struct ElementState<ViewSeqState> {
    child_states: ViewSeqState,
    child_elements: Vec<Pod>,
}

/// Create a new element view
///
/// If the element has no chilcdren, use the unit type (e.g. `let view = element("div", ())`).
pub fn element<E, ViewSeq>(
    name: impl Into<Cow<'static, str>>,
    children: ViewSeq,
) -> Element<E, ViewSeq> {
    Element {
        name: name.into(),
        attributes: BTreeMap::new(),
        children,
        ty: PhantomData,
    }
}

impl<E, ViewSeq> Element<E, ViewSeq> {
    /// Set an attribute on this element.
    ///
    /// # Panics
    ///
    /// If the name contains characters that are not valid in an attribute name,
    /// then the `View::build`/`View::rebuild` functions will panic for this view.
    pub fn attr(
        mut self,
        name: impl Into<Cow<'static, str>>,
        value: impl Into<Cow<'static, str>>,
    ) -> Self {
        self.set_attr(name, value);
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
        name: impl Into<Cow<'static, str>>,
        value: impl Into<Cow<'static, str>>,
    ) {
        self.attributes.insert(name.into(), value.into());
    }
}

impl<El, Children> ViewMarker for Element<El, Children> {}

impl<T, A, El, Children> View<T, A> for Element<El, Children>
where
    Children: ViewSequence<T, A>,
    // In addition, the `E` parameter is expected to be a child of `web_sys::Node`
    El: ElementTag,
{
    type State = ElementState<Children::State>;
    type Element = El::WebSysElement;

    fn build(&self, cx: &mut Cx) -> (Id, Self::State, Self::Element) {
        let el = cx.create_html_element(&self.name);
        for (name, value) in &self.attributes {
            el.set_attribute(name, value).unwrap();
        }
        let mut child_elements = vec![];
        let (id, child_states) = cx.with_new_id(|cx| self.children.build(cx, &mut child_elements));
        for child in &child_elements {
            el.append_child(child.0.as_node_ref()).unwrap();
        }
        let state = ElementState {
            child_states,
            child_elements,
        };
        (id, state, el.dyn_into().unwrap_throw())
    }

    fn rebuild(
        &self,
        cx: &mut Cx,
        prev: &Self,
        id: &mut Id,
        state: &mut Self::State,
        element: &mut Self::Element,
    ) -> ChangeFlags {
        let mut changed = ChangeFlags::empty();
        // update tag name
        if prev.name != self.name {
            // recreate element
            let parent = element
                .as_element_ref()
                .parent_element()
                .expect_throw("this element was mounted and so should have a parent");
            parent.remove_child(element.as_node_ref()).unwrap();
            let new_element = cx.create_html_element(&self.name);
            // TODO could this be combined with child updates?
            while element.as_element_ref().child_element_count() > 0 {
                new_element
                    .append_child(&element.as_element_ref().child_nodes().get(0).unwrap_throw())
                    .unwrap_throw();
            }
            *element = new_element.dyn_into().unwrap_throw();
            changed |= ChangeFlags::STRUCTURE;
        }

        let element = element.as_element_ref();

        // update attributes
        // TODO can I use VecSplice for this?
        let mut prev_attrs = prev.attributes.iter().peekable();
        let mut self_attrs = self.attributes.iter().peekable();
        while let (Some((prev_name, prev_value)), Some((self_name, self_value))) =
            (prev_attrs.peek(), self_attrs.peek())
        {
            match prev_name.cmp(self_name) {
                Ordering::Less => {
                    // attribute from prev is disappeared
                    remove_attribute(element, prev_name);
                    changed |= ChangeFlags::OTHER_CHANGE;
                    prev_attrs.next();
                }
                Ordering::Greater => {
                    // new attribute has appeared
                    set_attribute(element, self_name, self_value);
                    changed |= ChangeFlags::OTHER_CHANGE;
                    self_attrs.next();
                }
                Ordering::Equal => {
                    // attribute may has changed
                    if prev_value != self_value {
                        set_attribute(element, self_name, self_value);
                        changed |= ChangeFlags::OTHER_CHANGE;
                    }
                    prev_attrs.next();
                    self_attrs.next();
                }
            }
        }
        // Only max 1 of these loops will run
        while let Some((name, _)) = prev_attrs.next() {
            remove_attribute(element, name);
            changed |= ChangeFlags::OTHER_CHANGE;
        }
        while let Some((name, value)) = self_attrs.next() {
            set_attribute(element, name, value);
            changed |= ChangeFlags::OTHER_CHANGE;
        }

        // update children
        // TODO avoid reallocation every render?
        let mut scratch = vec![];
        let mut splice = VecSplice::new(&mut state.child_elements, &mut scratch);
        changed |= cx.with_id(*id, |cx| {
            self.children
                .rebuild(cx, &prev.children, &mut state.child_states, &mut splice)
        });
        if changed.contains(ChangeFlags::STRUCTURE) {
            // This is crude and will result in more DOM traffic than needed.
            // The right thing to do is diff the new state of the children id
            // vector against the old, and derive DOM mutations from that.
            while let Some(child) = element.first_child() {
                element.remove_child(&child).unwrap();
            }
            for child in &state.child_elements {
                element.append_child(child.0.as_node_ref()).unwrap();
            }
            changed.remove(ChangeFlags::STRUCTURE);
        }
        changed
    }

    fn message(
        &self,
        id_path: &[Id],
        state: &mut Self::State,
        message: Box<dyn std::any::Any>,
        app_state: &mut T,
    ) -> MessageResult<A> {
        self.children
            .message(id_path, &mut state.child_states, message, app_state)
    }
}

#[cfg(feature = "typed")]
fn set_attribute(element: &web_sys::Element, name: &str, value: &str) {
    // we have to special-case `value` because setting the value using `set_attribute`
    // doesn't work after the value has been changed.
    if name == "value" {
        let element: &web_sys::HtmlInputElement = element.dyn_ref().unwrap_throw();
        element.set_value(value)
    } else if name == "checked" {
        let element: &web_sys::HtmlInputElement = element.dyn_ref().unwrap_throw();
        element.set_checked(true)
    } else {
        element.set_attribute(name, value).unwrap_throw();
    }
}

#[cfg(not(feature = "typed"))]
fn set_attribute(element: &web_sys::Element, name: &str, value: &str) {
    element.set_attribute(name, value).unwrap_throw();
}

#[cfg(feature = "typed")]
fn remove_attribute(element: &web_sys::Element, name: &str) {
    // we have to special-case `value` because setting the value using `set_attribute`
    // doesn't work after the value has been changed.
    if name == "checked" {
        let element: &web_sys::HtmlInputElement = element.dyn_ref().unwrap_throw();
        element.set_checked(false)
    } else {
        element.remove_attribute(name).unwrap_throw();
    }
}

#[cfg(not(feature = "typed"))]
fn remove_attribute(element: &web_sys::Element, name: &str) {
    element.remove_attribute(name).unwrap_throw();
}
