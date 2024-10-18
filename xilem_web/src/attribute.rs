// Copyright 2024 the Xilem Authors
// SPDX-License-Identifier: Apache-2.0

use crate::{
    core::{MessageResult, Mut, View, ViewElement, ViewId, ViewMarker},
    vecmap::VecMap,
    AttributeValue, DomView, DynMessage, ElementProps, IntoAttributeValue, ViewCtx, With,
};
use std::marker::PhantomData;
use wasm_bindgen::{JsCast, UnwrapThrowExt};

type CowStr = std::borrow::Cow<'static, str>;

#[derive(Debug, PartialEq, Clone)]
/// An modifier element to either set or remove an attribute.
///
/// It's used in [`Attributes`].
pub enum AttributeModifier {
    Set(CowStr, AttributeValue),
    Remove(CowStr),
}

impl AttributeModifier {
    /// Returns the attribute name of this modifier.
    fn name(&self) -> &CowStr {
        let (AttributeModifier::Set(name, _) | AttributeModifier::Remove(name)) = self;
        name
    }

    /// Convert this modifier into its attribute name.
    fn into_name(self) -> CowStr {
        let (AttributeModifier::Set(name, _) | AttributeModifier::Remove(name)) = self;
        name
    }
}

impl<K: Into<CowStr>, V: IntoAttributeValue> From<(K, V)> for AttributeModifier {
    fn from((name, value): (K, V)) -> Self {
        match value.into_attr_value() {
            Some(value) => AttributeModifier::Set(name.into(), value),
            None => AttributeModifier::Remove(name.into()),
        }
    }
}

#[derive(Default)]
/// An Element modifier that manages all attributes of an Element.
pub struct Attributes {
    modifiers: Vec<AttributeModifier>,
    // TODO think about this (for a `VecSplice`) for more efficient insertion etc.,
    // but this is an additional trade-off of memory-usage and complexity,
    // while probably not helping much in the average case (of very few styles)...
    updated: VecMap<CowStr, ()>,
    idx: u16,
    in_hydration: bool,
    was_created: bool,
}

impl With<Attributes> for ElementProps {
    fn modifier(&mut self) -> &mut Attributes {
        self.attributes()
    }
}

impl Attributes {
    /// Creates a new `Attributes` modifier.
    ///
    /// `size_hint` is used to avoid unnecessary allocations while traversing up the view-tree when adding modifiers in [`View::build`].
    pub(crate) fn new(size_hint: usize, #[cfg(feature = "hydration")] in_hydration: bool) -> Self {
        Self {
            modifiers: Vec::with_capacity(size_hint),
            was_created: true,
            #[cfg(feature = "hydration")]
            in_hydration,
            ..Default::default()
        }
    }

    /// Applies potential changes of the attributes of an element to the underlying DOM node.
    pub fn apply_changes(&mut self, element: &web_sys::Element) {
        #[cfg(feature = "hydration")]
        if self.in_hydration {
            self.in_hydration = false;
            self.was_created = false;
        } else if self.was_created {
            for modifier in &self.modifiers {
                match modifier {
                    AttributeModifier::Remove(n) => remove_attribute(element, n),
                    AttributeModifier::Set(n, v) => set_attribute(element, n, &v.serialize()),
                }
            }
            self.was_created = false;
        } else if !self.updated.is_empty() {
            for modifier in self.modifiers.iter().rev() {
                match modifier {
                    AttributeModifier::Remove(name) if self.updated.remove(name).is_some() => {
                        remove_attribute(element, name);
                    }
                    AttributeModifier::Set(name, value) if self.updated.remove(name).is_some() => {
                        set_attribute(element, name, &value.serialize());
                    }
                    _ => {}
                }
            }
            // if there's any remaining key in updated, it means these are deleted keys
            for (name, ()) in self.updated.drain() {
                remove_attribute(element, &name);
            }
        }
        debug_assert!(self.updated.is_empty());
    }

    #[inline]
    /// Rebuilds the current element, while ensuring that the order of the modifiers stays correct.
    /// Any children should be rebuilt in inside `f`, *before* modifing any other properties of [`Attributes`].
    pub fn rebuild<E: With<Self>>(mut element: E, prev_len: usize, f: impl FnOnce(E)) {
        element.modifier().idx -= prev_len as u16;
        f(element);
    }

    #[inline]
    /// Returns whether the underlying element has been rebuilt, this could e.g. happen, when `OneOf` changes a variant to a different element.
    pub fn was_recreated(&self) -> bool {
        self.was_created
    }

    #[inline]
    /// Pushes `modifier` at the end of the current modifiers.
    pub fn push(&mut self, modifier: impl Into<AttributeModifier>) {
        let modifier = modifier.into();
        if !self.was_created && !self.in_hydration {
            self.updated.insert(modifier.name().clone(), ());
        }
        self.modifiers.push(modifier);
        self.idx += 1;
    }

    #[inline]
    /// Inserts `modifier` at the current index.
    pub fn insert(&mut self, modifier: impl Into<AttributeModifier>) {
        let modifier = modifier.into();
        self.updated.insert(modifier.name().clone(), ());
        // TODO this could potentially be expensive, maybe think about `VecSplice` again.
        // Although in the average case, this is likely not relevant, as usually very few attributes are used, thus shifting is probably good enough
        // I.e. a `VecSplice` is probably less optimal (either more complicated code, and/or more memory usage)
        self.modifiers.insert(self.idx as usize, modifier);
        self.idx += 1;
    }

    #[inline]
    /// Mutates the next modifier.
    pub fn mutate<R>(&mut self, f: impl FnOnce(&mut AttributeModifier) -> R) -> R {
        let modifier = &mut self.modifiers[self.idx as usize];
        let old = modifier.name().clone();
        let rv = f(modifier);
        let new = modifier.name();
        if *new != old {
            self.updated.insert(new.clone(), ());
        }
        self.updated.insert(old, ());
        self.idx += 1;
        rv
    }

    /// Skips the next `count` modifiers.
    pub fn skip(&mut self, count: usize) {
        self.idx += count as u16;
    }

    /// Deletes the next `count` modifiers.
    pub fn delete(&mut self, count: usize) {
        let start = self.idx as usize;
        for modifier in self.modifiers.drain(start..(start + count)) {
            self.updated.insert(modifier.into_name(), ());
        }
    }

    /// Updates the next modifier, based on the diff of `prev` and `next`.
    pub fn update(&mut self, prev: &AttributeModifier, next: &AttributeModifier) {
        if self.was_recreated() {
            self.push(next.clone());
        } else if next != prev {
            self.mutate(|modifier| *modifier = next.clone());
        } else {
            self.skip(1);
        }
    }

    /// Updates the next modifier, based on the diff of `prev` and `next`, this can be used only when the previous modifier has the same name `key`, and only its value has changed.
    pub fn update_with_same_key<Value: IntoAttributeValue + PartialEq + Clone>(
        &mut self,
        key: impl Into<CowStr>,
        prev: &Value,
        next: &Value,
    ) {
        if self.was_recreated() {
            self.push((key, next.clone()));
        } else if next != prev {
            self.mutate(|modifier| *modifier = (key, next.clone()).into());
        } else {
            self.skip(1);
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

/// A view to add an attribute to [`Element`](`crate::interfaces::Element`) derived components.
///
/// See [`Element::attr`](`crate::interfaces::Element::attr`) for more usage information.
pub struct Attr<V, State, Action> {
    inner: V,
    modifier: AttributeModifier,
    phantom: PhantomData<fn() -> (State, Action)>,
}

impl<V, State, Action> Attr<V, State, Action> {
    /// Create an [`Attr`] view. When `value` is `None`, it means remove the `name` attribute.
    ///
    /// Usually [`Element::attr`](`crate::interfaces::Element::attr`) should be used instead of this function.
    pub fn new(el: V, name: CowStr, value: Option<AttributeValue>) -> Self {
        let modifier = match value {
            Some(value) => AttributeModifier::Set(name, value),
            None => AttributeModifier::Remove(name),
        };
        Self {
            inner: el,
            modifier,
            phantom: PhantomData,
        }
    }
}

impl<V, State, Action> ViewMarker for Attr<V, State, Action> {}
impl<V, State, Action> View<State, Action, ViewCtx, DynMessage> for Attr<V, State, Action>
where
    State: 'static,
    Action: 'static,
    V: DomView<State, Action, Element: With<Attributes>>,
    for<'a> <V::Element as ViewElement>::Mut<'a>: With<Attributes>,
{
    type Element = V::Element;

    type ViewState = V::ViewState;

    fn build(&self, ctx: &mut ViewCtx) -> (Self::Element, Self::ViewState) {
        let (mut element, state) =
            ctx.with_size_hint::<Attributes, _>(1, |ctx| self.inner.build(ctx));
        element.modifier().push(self.modifier.clone());
        (element, state)
    }

    fn rebuild(
        &self,
        prev: &Self,
        view_state: &mut Self::ViewState,
        ctx: &mut ViewCtx,
        element: Mut<Self::Element>,
    ) {
        Attributes::rebuild(element, 1, |mut element| {
            self.inner
                .rebuild(&prev.inner, view_state, ctx, element.reborrow_mut());

            let attrs = element.modifier();
            attrs.update(&prev.modifier, &self.modifier);
        });
    }

    fn teardown(
        &self,
        view_state: &mut Self::ViewState,
        ctx: &mut ViewCtx,
        element: Mut<Self::Element>,
    ) {
        self.inner.teardown(view_state, ctx, element);
    }

    fn message(
        &self,
        view_state: &mut Self::ViewState,
        id_path: &[ViewId],
        message: DynMessage,
        app_state: &mut State,
    ) -> MessageResult<Action, DynMessage> {
        self.inner.message(view_state, id_path, message, app_state)
    }
}
