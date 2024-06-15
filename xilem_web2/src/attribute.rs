use std::marker::PhantomData;
use wasm_bindgen::{JsCast, UnwrapThrowExt};
use xilem_core::{DynMessage, MessageResult, Mut, View, ViewElement, ViewId};

use crate::{vecmap::VecMap, AttributeValue, DomNode, ElementProps, Pod, PodMut, ViewCtx};

type CowStr = std::borrow::Cow<'static, str>;

pub trait WithAttributes {
    fn start_attribute_modifier(&mut self);
    fn end_attribute_modifier(&mut self);
    fn set_attribute(&mut self, name: CowStr, value: Option<AttributeValue>);
    // TODO first find a use-case for this...
    // fn get_attr(&self, name: &str) -> Option<&AttributeValue>;
}

#[derive(Debug, PartialEq)]
enum AttributeModifier {
    Remove(CowStr),
    Set(CowStr, AttributeValue),
    EndMarker(usize),
}

#[derive(Debug, Default)]
pub struct Attributes {
    attribute_modifiers: Vec<AttributeModifier>,
    updated_attributes: VecMap<CowStr, ()>,
    idx: usize, // To save some memory, this could be u16 or even u8 (but this is risky)
    start_idx: usize, // same here
    /// a flag necessary, such that `start_attribute_modifier` doesn't always overwrite the last changes in `View::build`
    build_finished: bool,
}

fn set_attribute(element: &web_sys::Element, name: &str, value: &str) {
    // we have to special-case `value` because setting the value using `set_attribute`
    // doesn't work after the value has been changed.
    if name == "value" {
        // TODO not sure, whether this is always a good idea, in case custom elements are used that have "value" as an attribute name.
        let element: &web_sys::HtmlInputElement = element.dyn_ref().unwrap_throw();
        element.set_value(value);
    } else if name == "checked" {
        let element: &web_sys::HtmlInputElement = element.dyn_ref().unwrap_throw();
        element.set_checked(true);
    } else {
        element.set_attribute(name, value).unwrap_throw();
    }
}

fn remove_attribute(element: &web_sys::Element, name: &str) {
    // we have to special-case `checked` because setting the value using `set_attribute`
    // doesn't work after the value has been changed.
    if name == "checked" {
        let element: &web_sys::HtmlInputElement = element.dyn_ref().unwrap_throw();
        element.set_checked(false);
    } else {
        element.remove_attribute(name).unwrap_throw();
    }
}

impl Attributes {
    pub fn apply_attribute_changes(&mut self, element: &web_sys::Element) {
        if !self.updated_attributes.is_empty() {
            for modifier in self.attribute_modifiers.iter().rev() {
                match modifier {
                    AttributeModifier::Remove(name) => {
                        if self.updated_attributes.contains_key(name) {
                            self.updated_attributes.remove(name);
                            remove_attribute(element, name);
                            // element.remove_attribute(name);
                        }
                    }
                    AttributeModifier::Set(name, value) => {
                        if self.updated_attributes.contains_key(name) {
                            self.updated_attributes.remove(name);
                            set_attribute(element, name, &value.serialize());
                            // element.set_attribute(name, &value.serialize());
                        }
                    }
                    AttributeModifier::EndMarker(_) => (),
                }
            }
            debug_assert!(self.updated_attributes.is_empty());
        }
        self.build_finished = true;
    }
}

impl WithAttributes for Attributes {
    fn set_attribute(&mut self, name: CowStr, value: Option<AttributeValue>) {
        let new_modifier = if let Some(value) = value {
            AttributeModifier::Set(name.clone(), value)
        } else {
            AttributeModifier::Remove(name.clone())
        };

        if let Some(modifier) = self.attribute_modifiers.get_mut(self.idx) {
            if modifier != &new_modifier {
                if let AttributeModifier::Remove(previous_name)
                | AttributeModifier::Set(previous_name, _) = modifier
                {
                    if &name != previous_name {
                        self.updated_attributes.insert(previous_name.clone(), ());
                    }
                }
                self.updated_attributes.insert(name, ());
                *modifier = new_modifier;
            }
            // else remove it out of updated_attributes? (because previous attributes are overwritten) not sure if worth it because potentially worse perf
        } else {
            self.updated_attributes.insert(name, ());
            self.attribute_modifiers.push(new_modifier);
        }
        self.idx += 1;
    }

    fn start_attribute_modifier(&mut self) {
        if self.build_finished {
            if self.idx == 0 {
                self.start_idx = 0;
            } else {
                let AttributeModifier::EndMarker(start_idx) =
                    self.attribute_modifiers[self.idx - 1]
                else {
                    unreachable!("this should not happen, as either `start_attribute_modifier` happens first, or follows an end_attribute_modifier")
                };
                self.idx = start_idx;
                self.start_idx = start_idx;
            }
        }
    }

    fn end_attribute_modifier(&mut self) {
        match self.attribute_modifiers.get_mut(self.idx) {
            Some(AttributeModifier::EndMarker(prev_start_idx))
                if *prev_start_idx == self.start_idx => {} // class modifier hasn't changed
            Some(modifier) => {
                *modifier = AttributeModifier::EndMarker(self.start_idx);
            }
            None => {
                self.attribute_modifiers
                    .push(AttributeModifier::EndMarker(self.start_idx));
            }
        }
        self.idx += 1;
        self.start_idx = self.idx;
    }
}

impl WithAttributes for ElementProps {
    fn start_attribute_modifier(&mut self) {
        self.attributes().start_attribute_modifier();
    }

    fn end_attribute_modifier(&mut self) {
        self.attributes().end_attribute_modifier();
    }

    fn set_attribute(&mut self, name: CowStr, value: Option<AttributeValue>) {
        self.attributes().set_attribute(name, value);
    }
}

impl<E: DomNode<P>, P: WithAttributes> WithAttributes for Pod<E, P> {
    fn start_attribute_modifier(&mut self) {
        self.props.start_attribute_modifier();
    }

    fn end_attribute_modifier(&mut self) {
        self.props.end_attribute_modifier();
    }

    fn set_attribute(&mut self, name: CowStr, value: Option<AttributeValue>) {
        self.props.set_attribute(name, value);
    }
}

impl<E: DomNode<P>, P: WithAttributes> WithAttributes for PodMut<'_, E, P> {
    fn start_attribute_modifier(&mut self) {
        self.props.start_attribute_modifier();
    }

    fn end_attribute_modifier(&mut self) {
        self.props.end_attribute_modifier();
    }

    fn set_attribute(&mut self, name: CowStr, value: Option<AttributeValue>) {
        self.props.set_attribute(name, value);
    }
}

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

impl<T, A, E> View<T, A, ViewCtx> for Attr<E, T, A>
where
    T: 'static,
    A: 'static,
    E: View<T, A, ViewCtx, Element: ElementWithAttributes>,
{
    type Element = E::Element;

    type ViewState = E::ViewState;

    fn build(&self, ctx: &mut ViewCtx) -> (Self::Element, Self::ViewState) {
        let (mut element, state) = self.el.build(ctx);
        element.start_attribute_modifier();
        element.set_attribute(self.name.clone(), self.value.clone());
        element.end_attribute_modifier();
        (element, state)
    }

    fn rebuild<'e>(
        &self,
        prev: &Self,
        view_state: &mut Self::ViewState,
        ctx: &mut ViewCtx,
        mut element: Mut<'e, Self::Element>,
    ) -> Mut<'e, Self::Element> {
        element.start_attribute_modifier();
        let mut element = self.el.rebuild(&prev.el, view_state, ctx, element);
        element.set_attribute(self.name.clone(), self.value.clone());
        element.end_attribute_modifier();
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
    ) -> MessageResult<A> {
        self.el.message(view_state, id_path, message, app_state)
    }
}
