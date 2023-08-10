//! The HTML element view and associated types/functions.
//!
//! If you are writing your own views, we recommend adding
//! `use xilem_html::elements as el` or similar to the top of your file.
use crate::{
    context::{ChangeFlags, Cx},
    diff::Diff,
    vecmap::VecMap,
    view::{DomElement, Pod, View, ViewMarker, ViewSequence},
};

use std::{borrow::Cow, fmt};
use wasm_bindgen::{JsCast, UnwrapThrowExt};
use xilem_core::{Id, MessageResult, VecSplice};

#[cfg(feature = "typed")]
pub mod elements;

/// A view representing a HTML element.
///
/// If the element has no children, use the unit type (e.g. `let view = element("div", ())`).
pub struct Element<El, Children = ()> {
    name: Cow<'static, str>,
    attributes: VecMap<Cow<'static, str>, Cow<'static, str>>,
    children: Children,
    #[allow(clippy::type_complexity)]
    after_update: Option<Box<dyn Fn(&El)>>,
}

impl<El, ViewSeq> Element<El, ViewSeq> {
    pub fn debug_as_el(&self) -> impl fmt::Debug + '_ {
        struct DebugFmt<'a, El, VS>(&'a Element<El, VS>);
        impl<'a, El, VS> fmt::Debug for DebugFmt<'a, El, VS> {
            fn fmt(&self, f: &mut fmt::Formatter) -> fmt::Result {
                write!(f, "<{}", self.0.name)?;
                for (name, value) in self.0.attributes.iter() {
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
    scratch: Vec<Pod>,
}

/// Create a new element view
///
/// If the element has no children, use the unit type (e.g. `let view = element("div", ())`).
pub fn element<El, ViewSeq>(
    name: impl Into<Cow<'static, str>>,
    children: ViewSeq,
) -> Element<El, ViewSeq> {
    Element {
        name: name.into(),
        attributes: VecMap::default(),
        children,
        after_update: None,
    }
}

impl<El, ViewSeq> Element<El, ViewSeq> {
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

    /// Set a function to run after the new view tree has been created.
    ///
    /// This offers functionality similar to `ref` in React.
    ///
    /// # Rules for correct use
    ///
    /// It is important that the structure of the DOM tree is *not* modified using this function.
    /// If the DOM tree is modified, then future reconciliation will have undefined and possibly
    /// suprising results.
    pub fn after_update(mut self, after_update: impl Fn(&El) + 'static) -> Self {
        self.after_update = Some(Box::new(after_update));
        self
    }
}

impl<El, Children> ViewMarker for Element<El, Children> {}

impl<T, A, El, Children> View<T, A> for Element<El, Children>
where
    Children: ViewSequence<T, A>,
    El: JsCast + DomElement,
{
    type State = ElementState<Children::State>;
    type Element = El;

    fn build(&self, cx: &mut Cx) -> (Id, Self::State, Self::Element) {
        let el = cx.create_html_element(&self.name);
        for (name, value) in self.attributes.iter() {
            el.set_attribute(name, value).unwrap_throw();
        }
        let mut child_elements = vec![];
        let (id, child_states) = cx.with_new_id(|cx| self.children.build(cx, &mut child_elements));
        for child in &child_elements {
            el.append_child(child.0.as_node_ref()).unwrap_throw();
        }

        // Set the id used internally to the `data-debugid` attribute.
        // This allows the user to see if an element has been re-created or only altered.
        #[cfg(debug_assertions)]
        el.set_attribute("data-debugid", &id.to_raw().to_string())
            .unwrap_throw();

        let el = el.dyn_into().unwrap_throw();
        if let Some(after_update) = &self.after_update {
            (after_update)(&el);
        }
        let state = ElementState {
            child_states,
            child_elements,
            scratch: vec![],
        };
        (id, state, el)
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
            parent.remove_child(element.as_node_ref()).unwrap_throw();
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
        for itm in prev.attributes.diff(&self.attributes) {
            match itm {
                Diff::Add(name, value) | Diff::Change(name, value) => {
                    set_attribute(element, name, value);
                    changed |= ChangeFlags::OTHER_CHANGE;
                }
                Diff::Remove(name) => {
                    remove_attribute(element, name);
                    changed |= ChangeFlags::OTHER_CHANGE;
                }
            }
        }

        // update children
        let mut splice = VecSplice::new(&mut state.child_elements, &mut state.scratch);
        changed |= cx.with_id(*id, |cx| {
            self.children
                .rebuild(cx, &prev.children, &mut state.child_states, &mut splice)
        });
        if changed.contains(ChangeFlags::STRUCTURE) {
            // This is crude and will result in more DOM traffic than needed.
            // The right thing to do is diff the new state of the children id
            // vector against the old, and derive DOM mutations from that.
            while let Some(child) = element.first_child() {
                element.remove_child(&child).unwrap_throw();
            }
            for child in &state.child_elements {
                element.append_child(child.0.as_node_ref()).unwrap_throw();
            }
            changed.remove(ChangeFlags::STRUCTURE);
        }
        if let Some(after_update) = &self.after_update {
            (after_update)(element.dyn_ref().unwrap_throw());
            changed |= ChangeFlags::OTHER_CHANGE;
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
