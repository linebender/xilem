// Copyright 2024 the Xilem Authors
// SPDX-License-Identifier: Apache-2.0

use std::marker::PhantomData;
use wasm_bindgen::{JsCast, UnwrapThrowExt};
use xilem_core::{MessageResult, Mut, View, ViewElement, ViewId, ViewMarker};

use crate::{
    vecmap::VecMap, AttributeValue, DomNode, DynMessage, ElementProps, Pod, PodMut, ViewCtx,
};

type CowStr = std::borrow::Cow<'static, str>;

/// This trait allows (modifying) HTML/SVG/MathML attributes on DOM [`Element`](`crate::interfaces::Element`)s.
///
/// Modifications have to be done on the up-traversal of [`View::rebuild`], i.e. after [`View::rebuild`] was invoked for descendent views.
/// See [`Attr::build`] and [`Attr::rebuild`], how to use this for [`ViewElement`]s that implement this trait.
/// When these methods are used, they have to be used in every reconciliation pass (i.e. [`View::rebuild`]).
pub trait WithAttributes {
    /// Needs to be invoked within a [`View::rebuild`] before traversing to descendent views, and before any modifications (with [`set_attribute`](`WithAttributes::set_attribute`)) are done in that view
    fn rebuild_attribute_modifier(&mut self);

    /// Needs to be invoked after any modifications are done
    fn mark_end_of_attribute_modifier(&mut self);

    /// Sets or removes (when value is `None`) an attribute from the underlying element.
    ///
    /// When in [`View::rebuild`] this has to be invoked *after* traversing the inner `View` with [`View::rebuild`]
    fn set_attribute(&mut self, name: &CowStr, value: &Option<AttributeValue>);

    // TODO first find a use-case for this...
    // fn get_attr(&self, name: &str) -> Option<&AttributeValue>;
}

#[derive(Debug, PartialEq)]
enum AttributeModifier {
    Remove(CowStr),
    Set(CowStr, AttributeValue),
    EndMarker(u16),
}

const HYDRATING: u16 = 1 << 14;
const CREATING: u16 = 1 << 15;
const RESERVED_BIT_MASK: u16 = HYDRATING | CREATING;

/// This contains all the current attributes of an [`Element`](`crate::interfaces::Element`)
#[derive(Debug, Default)]
pub struct Attributes {
    attribute_modifiers: Vec<AttributeModifier>,
    updated_attributes: VecMap<CowStr, ()>,
    idx: u16,
    /// the two most significant bits are reserved for whether this was just created (bit 15) and if it's currently being hydrated (bit 14)
    start_idx: u16,
}

impl Attributes {
    pub(crate) fn new(size_hint: usize, #[cfg(feature = "hydration")] in_hydration: bool) -> Self {
        #[allow(unused_mut)]
        let mut start_idx = CREATING;
        #[cfg(feature = "hydration")]
        if in_hydration {
            start_idx |= HYDRATING;
        }

        Self {
            attribute_modifiers: Vec::with_capacity(size_hint),
            start_idx,
            ..Default::default()
        }
    }
}

fn set_attribute(element: &web_sys::Element, name: &str, value: &str) {
    debug_assert_ne!(
        name, "class",
        "Using `class` as attribute is not supported, use the `el.class()` modifier instead"
    );
    debug_assert_ne!(
        name, "style",
        "Using `style` as attribute is not supported, use the `el.style()` modifier instead"
    );

    // we have to special-case `value` because setting the value using `set_attribute`
    // doesn't work after the value has been changed.
    // TODO not sure, whether this is always a good idea, in case custom or other interfaces such as HtmlOptionElement elements are used that have "value" as an attribute name.
    // We likely want to use the DOM attributes instead.
    if name == "value" {
        if let Some(input_element) = element.dyn_ref::<web_sys::HtmlInputElement>() {
            input_element.set_value(value);
        } else {
            element.set_attribute("value", value).unwrap_throw();
        }
    } else if name == "checked" {
        if let Some(input_element) = element.dyn_ref::<web_sys::HtmlInputElement>() {
            input_element.set_checked(true);
        } else {
            element.set_attribute("checked", value).unwrap_throw();
        }
    } else {
        element.set_attribute(name, value).unwrap_throw();
    }
}

fn remove_attribute(element: &web_sys::Element, name: &str) {
    debug_assert_ne!(
        name, "class",
        "Using `class` as attribute is not supported, use the `el.class()` modifier instead"
    );
    debug_assert_ne!(
        name, "style",
        "Using `style` as attribute is not supported, use the `el.style()` modifier instead"
    );
    // we have to special-case `checked` because setting the value using `set_attribute`
    // doesn't work after the value has been changed.
    if name == "checked" {
        if let Some(input_element) = element.dyn_ref::<web_sys::HtmlInputElement>() {
            input_element.set_checked(false);
        } else {
            element.remove_attribute("checked").unwrap_throw();
        }
    } else {
        element.remove_attribute(name).unwrap_throw();
    }
}

impl Attributes {
    /// applies potential changes of the attributes of an element to the underlying DOM node
    pub fn apply_attribute_changes(&mut self, element: &web_sys::Element) {
        if (self.start_idx & HYDRATING) == HYDRATING {
            self.start_idx &= !RESERVED_BIT_MASK;
            return;
        }

        if (self.start_idx & CREATING) == CREATING {
            for modifier in self.attribute_modifiers.iter().rev() {
                match modifier {
                    AttributeModifier::Remove(name) => {
                        remove_attribute(element, name);
                    }
                    AttributeModifier::Set(name, value) => {
                        set_attribute(element, name, &value.serialize());
                    }
                    AttributeModifier::EndMarker(_) => (),
                }
            }
            self.start_idx &= !RESERVED_BIT_MASK;
            debug_assert!(self.updated_attributes.is_empty());
            return;
        }

        if !self.updated_attributes.is_empty() {
            for modifier in self.attribute_modifiers.iter().rev() {
                match modifier {
                    AttributeModifier::Remove(name) => {
                        if self.updated_attributes.remove(name).is_some() {
                            remove_attribute(element, name);
                        }
                    }
                    AttributeModifier::Set(name, value) => {
                        if self.updated_attributes.remove(name).is_some() {
                            set_attribute(element, name, &value.serialize());
                        }
                    }
                    AttributeModifier::EndMarker(_) => (),
                }
            }
            debug_assert!(self.updated_attributes.is_empty());
        }
    }
}

impl WithAttributes for Attributes {
    fn set_attribute(&mut self, name: &CowStr, value: &Option<AttributeValue>) {
        if (self.start_idx & RESERVED_BIT_MASK) != 0 {
            let modifier = if let Some(value) = value {
                AttributeModifier::Set(name.clone(), value.clone())
            } else {
                AttributeModifier::Remove(name.clone())
            };
            self.attribute_modifiers.push(modifier);
        } else if let Some(modifier) = self.attribute_modifiers.get_mut(self.idx as usize) {
            let dirty = match (&modifier, value) {
                // early return if nothing has changed, avoids allocations
                (AttributeModifier::Set(old_name, old_value), Some(new_value))
                    if old_name == name =>
                {
                    if old_value == new_value {
                        false
                    } else {
                        self.updated_attributes.insert(name.clone(), ());
                        true
                    }
                }
                (AttributeModifier::Remove(removed), None) if removed == name => false,
                (AttributeModifier::Set(old_name, _), None)
                | (AttributeModifier::Remove(old_name), Some(_))
                    if old_name == name =>
                {
                    self.updated_attributes.insert(name.clone(), ());
                    true
                }
                (AttributeModifier::EndMarker(_), None)
                | (AttributeModifier::EndMarker(_), Some(_)) => {
                    self.updated_attributes.insert(name.clone(), ());
                    true
                }
                (AttributeModifier::Set(old_name, _), _)
                | (AttributeModifier::Remove(old_name), _) => {
                    self.updated_attributes.insert(name.clone(), ());
                    self.updated_attributes.insert(old_name.clone(), ());
                    true
                }
            };
            if dirty {
                *modifier = if let Some(value) = value {
                    AttributeModifier::Set(name.clone(), value.clone())
                } else {
                    AttributeModifier::Remove(name.clone())
                };
            }
            // else remove it out of updated_attributes? (because previous attributes are overwritten) not sure if worth it because potentially worse perf
        } else {
            let new_modifier = if let Some(value) = value {
                AttributeModifier::Set(name.clone(), value.clone())
            } else {
                AttributeModifier::Remove(name.clone())
            };
            self.updated_attributes.insert(name.clone(), ());
            self.attribute_modifiers.push(new_modifier);
        }
        self.idx += 1;
    }

    fn rebuild_attribute_modifier(&mut self) {
        if self.idx == 0 {
            self.start_idx &= RESERVED_BIT_MASK;
        } else {
            let AttributeModifier::EndMarker(start_idx) =
                self.attribute_modifiers[(self.idx - 1) as usize]
            else {
                unreachable!("this should not happen, as either `rebuild_attribute_modifier` happens first, or follows an `mark_end_of_attribute_modifier`")
            };
            self.idx = start_idx;
            self.start_idx = start_idx | (self.start_idx & RESERVED_BIT_MASK);
        }
    }

    fn mark_end_of_attribute_modifier(&mut self) {
        let start_idx = self.start_idx & !RESERVED_BIT_MASK;
        match self.attribute_modifiers.get_mut(self.idx as usize) {
            Some(AttributeModifier::EndMarker(prev_start_idx)) if *prev_start_idx == start_idx => {} // attribute modifier hasn't changed
            Some(modifier) => *modifier = AttributeModifier::EndMarker(start_idx),
            None => self
                .attribute_modifiers
                .push(AttributeModifier::EndMarker(start_idx)),
        }
        self.idx += 1;
        self.start_idx = self.idx | (self.start_idx & RESERVED_BIT_MASK);
    }
}

impl WithAttributes for ElementProps {
    fn rebuild_attribute_modifier(&mut self) {
        self.attributes().rebuild_attribute_modifier();
    }

    fn mark_end_of_attribute_modifier(&mut self) {
        self.attributes().mark_end_of_attribute_modifier();
    }

    fn set_attribute(&mut self, name: &CowStr, value: &Option<AttributeValue>) {
        self.attributes().set_attribute(name, value);
    }
}

impl<N> WithAttributes for Pod<N>
where
    N: DomNode,
    N::Props: WithAttributes,
{
    fn rebuild_attribute_modifier(&mut self) {
        self.props.rebuild_attribute_modifier();
    }

    fn mark_end_of_attribute_modifier(&mut self) {
        self.props.mark_end_of_attribute_modifier();
    }

    fn set_attribute(&mut self, name: &CowStr, value: &Option<AttributeValue>) {
        self.props.set_attribute(name, value);
    }
}

impl<N> WithAttributes for PodMut<'_, N>
where
    N: DomNode,
    N::Props: WithAttributes,
{
    fn rebuild_attribute_modifier(&mut self) {
        self.props.rebuild_attribute_modifier();
    }

    fn mark_end_of_attribute_modifier(&mut self) {
        self.props.mark_end_of_attribute_modifier();
    }

    fn set_attribute(&mut self, name: &CowStr, value: &Option<AttributeValue>) {
        self.props.set_attribute(name, value);
    }
}

/// Syntax sugar for adding a type bound on the `ViewElement` of a view, such that both, [`ViewElement`] and [`ViewElement::Mut`] are bound to [`WithAttributes`]
pub trait ElementWithAttributes:
    for<'a> ViewElement<Mut<'a>: WithAttributes> + WithAttributes
{
}

impl<T> ElementWithAttributes for T
where
    T: ViewElement + WithAttributes,
    for<'a> T::Mut<'a>: WithAttributes,
{
}

/// A view to add or remove an attribute to/from an element, see [`Element::attr`](`crate::interfaces::Element::attr`) for how it's usually used.
#[derive(Clone, Debug)]
pub struct Attr<E, T, A> {
    el: E,
    name: CowStr,
    value: Option<AttributeValue>,
    phantom: PhantomData<fn() -> (T, A)>,
}

impl<E, T, A> Attr<E, T, A> {
    pub fn new(el: E, name: CowStr, value: Option<AttributeValue>) -> Self {
        Attr {
            el,
            name,
            value,
            phantom: PhantomData,
        }
    }
}

impl<E, T, A> ViewMarker for Attr<E, T, A> {}
impl<E, T, A> View<T, A, ViewCtx, DynMessage> for Attr<E, T, A>
where
    T: 'static,
    A: 'static,
    E: View<T, A, ViewCtx, DynMessage, Element: ElementWithAttributes>,
{
    type Element = E::Element;

    type ViewState = E::ViewState;

    fn build(&self, ctx: &mut ViewCtx) -> (Self::Element, Self::ViewState) {
        ctx.add_modifier_size_hint::<Attributes>(1);
        let (mut element, state) = self.el.build(ctx);
        element.set_attribute(&self.name, &self.value);
        element.mark_end_of_attribute_modifier();
        (element, state)
    }

    fn rebuild<'e>(
        &self,
        prev: &Self,
        view_state: &mut Self::ViewState,
        ctx: &mut ViewCtx,
        mut element: Mut<'e, Self::Element>,
    ) -> Mut<'e, Self::Element> {
        element.rebuild_attribute_modifier();
        let mut element = self.el.rebuild(&prev.el, view_state, ctx, element);
        element.set_attribute(&self.name, &self.value);
        element.mark_end_of_attribute_modifier();
        element
    }

    fn teardown(
        &self,
        view_state: &mut Self::ViewState,
        ctx: &mut ViewCtx,
        element: Mut<'_, Self::Element>,
    ) {
        self.el.teardown(view_state, ctx, element);
    }

    fn message(
        &self,
        view_state: &mut Self::ViewState,
        id_path: &[ViewId],
        message: DynMessage,
        app_state: &mut T,
    ) -> MessageResult<A, DynMessage> {
        self.el.message(view_state, id_path, message, app_state)
    }
}
