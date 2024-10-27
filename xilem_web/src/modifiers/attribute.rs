// Copyright 2024 the Xilem Authors
// SPDX-License-Identifier: Apache-2.0

use crate::{
    core::{MessageResult, Mut, View, ViewElement, ViewId, ViewMarker},
    modifiers::{Modifier, With},
    vecmap::VecMap,
    AttributeValue, DomView, DynMessage, IntoAttributeValue, ViewCtx,
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
    pub fn name(&self) -> &CowStr {
        let (AttributeModifier::Set(name, _) | AttributeModifier::Remove(name)) = self;
        name
    }

    /// Convert this modifier into its attribute name.
    pub fn into_name(self) -> CowStr {
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
    // TODO think about using a `VecSplice` for more efficient insertion etc.,
    // but this is an additional trade-off of memory-usage and complexity,
    // while probably not helping much in the average case (of very few styles)...
    modifiers: Vec<AttributeModifier>,
    updated: VecMap<CowStr, ()>,
    idx: usize,
}

impl Attributes {
    /// Creates a new `Attributes` modifier.
    ///
    /// `size_hint` is used to avoid unnecessary allocations while traversing up the view-tree when adding modifiers in [`View::build`].
    pub(crate) fn new(size_hint: usize) -> Self {
        Self {
            modifiers: Vec::with_capacity(size_hint),
            ..Default::default()
        }
    }

    /// Applies potential changes of the attributes of an element to the underlying DOM node.
    pub fn apply_changes(this: Modifier<'_, Self>, element: &web_sys::Element) {
        let Modifier { modifier, flags } = this;

        if !flags.in_hydration() && flags.was_created() {
            for modifier in &modifier.modifiers {
                match modifier {
                    AttributeModifier::Remove(n) => remove_attribute(element, n),
                    AttributeModifier::Set(n, v) => set_attribute(element, n, &v.serialize()),
                }
            }
        } else if !modifier.updated.is_empty() {
            for m in modifier.modifiers.iter().rev() {
                match m {
                    AttributeModifier::Remove(name) if modifier.updated.remove(name).is_some() => {
                        remove_attribute(element, name);
                    }
                    AttributeModifier::Set(name, value)
                        if modifier.updated.remove(name).is_some() =>
                    {
                        set_attribute(element, name, &value.serialize());
                    }
                    _ => {}
                }
            }
            // if there's any remaining key in updated, it means these are deleted keys
            for (name, ()) in modifier.updated.drain() {
                remove_attribute(element, &name);
            }
        }
        debug_assert!(modifier.updated.is_empty());
    }

    #[inline]
    /// Rebuilds the current element, while ensuring that the order of the modifiers stays correct.
    /// Any children should be rebuilt in inside `f`, *before* modifying any other properties of [`Attributes`].
    pub fn rebuild<E: With<Self>>(mut element: E, prev_len: usize, f: impl FnOnce(E)) {
        element.modifier().modifier.idx -= prev_len;
        f(element);
    }

    #[inline]
    /// Pushes `modifier` at the end of the current modifiers.
    ///
    /// Must only be used when `this.flags.was_created() == true`.
    pub fn push(this: &mut Modifier<'_, Self>, modifier: impl Into<AttributeModifier>) {
        debug_assert!(
            this.flags.was_created(),
            "This should never be called, when the underlying element wasn't (re)created. Use `Attributes::insert` instead."
        );
        this.flags.set_needs_update();
        this.modifier.modifiers.push(modifier.into());
        this.modifier.idx += 1;
    }

    #[inline]
    /// Inserts `modifier` at the current index.
    ///
    /// Must only be used when `this.flags.was_created() == false`.
    pub fn insert(this: &mut Modifier<'_, Self>, modifier: impl Into<AttributeModifier>) {
        debug_assert!(
            !this.flags.was_created(),
            "This should never be called, when the underlying element was (re)created, use `Attributes::push` instead."
        );
        this.flags.set_needs_update();
        let modifier = modifier.into();
        this.modifier.updated.insert(modifier.name().clone(), ());
        // TODO this could potentially be expensive, maybe think about `VecSplice` again.
        // Although in the average case, this is likely not relevant, as usually very few attributes are used, thus shifting is probably good enough
        // I.e. a `VecSplice` is probably less optimal (either more complicated code, and/or more memory usage)
        this.modifier.modifiers.insert(this.modifier.idx, modifier);
        this.modifier.idx += 1;
    }

    #[inline]
    /// Mutates the next modifier.
    ///
    /// Must only be used when `this.flags.was_created() == false`.
    pub fn mutate<R>(
        this: &mut Modifier<'_, Self>,
        f: impl FnOnce(&mut AttributeModifier) -> R,
    ) -> R {
        debug_assert!(
            !this.flags.was_created(),
            "This should never be called, when the underlying element was (re)created."
        );
        this.flags.set_needs_update();
        let modifier = &mut this.modifier.modifiers[this.modifier.idx];
        let old = modifier.name().clone();
        let rv = f(modifier);
        let new = modifier.name();
        if *new != old {
            this.modifier.updated.insert(new.clone(), ());
        }
        this.modifier.updated.insert(old, ());
        this.modifier.idx += 1;
        rv
    }

    /// Skips the next `count` modifiers.
    ///
    /// Must only be used when `this.flags.was_created() == false`.
    pub fn skip(this: &mut Modifier<'_, Self>, count: usize) {
        debug_assert!(
            !this.flags.was_created(),
            "This should never be called, when the underlying element was (re)created."
        );
        this.modifier.idx += count;
    }

    /// Deletes the next `count` modifiers.
    ///
    /// Must only be used when `this.flags.was_created() == false`.
    pub fn delete(this: &mut Modifier<'_, Self>, count: usize) {
        debug_assert!(
            !this.flags.was_created(),
            "This should never be called, when the underlying element was (re)created."
        );
        let start = this.modifier.idx;
        this.flags.set_needs_update();
        for modifier in this.modifier.modifiers.drain(start..(start + count)) {
            this.modifier.updated.insert(modifier.into_name(), ());
        }
    }

    /// Updates the next modifier, based on the diff of `prev` and `next`.
    pub fn update(
        this: &mut Modifier<'_, Self>,
        prev: &AttributeModifier,
        next: &AttributeModifier,
    ) {
        if this.flags.was_created() {
            Attributes::push(this, next.clone());
        } else if next != prev {
            Attributes::mutate(this, |modifier| *modifier = next.clone());
        } else {
            Attributes::skip(this, 1);
        }
    }

    /// Updates the next modifier, based on the diff of `prev` and `next`, this can be used only when the previous modifier has the same name `key`, and only its value has changed.
    pub fn update_with_same_key<Value: IntoAttributeValue + PartialEq + Clone>(
        this: &mut Modifier<'_, Self>,
        key: impl Into<CowStr>,
        prev: &Value,
        next: &Value,
    ) {
        if this.flags.was_created() {
            Attributes::push(this, (key, next.clone()));
        } else if next != prev {
            Attributes::mutate(this, |modifier| *modifier = (key, next.clone()).into());
        } else {
            Attributes::skip(this, 1);
        }
    }
}

fn html_input_attr_assertions(element: &web_sys::Element, name: &str) {
    debug_assert!(
        !(element.is_instance_of::<web_sys::HtmlInputElement>() && name == "checked"),
        "Using `checked` as attribute on a checkbox is not supported, \
         use the `el.checked()` or `el.default_checked()` modifier instead."
    );
    debug_assert!(
        !(element.is_instance_of::<web_sys::HtmlInputElement>() && name == "disabled"),
        "Using `disabled` as attribute on an input element is not supported, \
         use the `el.checked()` modifier instead."
    );
}

fn element_attr_assertions(element: &web_sys::Element, name: &str) {
    debug_assert_ne!(
        name, "class",
        "Using `class` as attribute is not supported, use the `el.class()` modifier instead"
    );
    debug_assert_ne!(
        name, "style",
        "Using `style` as attribute is not supported, use the `el.style()` modifier instead"
    );
    html_input_attr_assertions(element, name);
}

fn set_attribute(element: &web_sys::Element, name: &str, value: &str) {
    element_attr_assertions(element, name);

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
    } else {
        element.set_attribute(name, value).unwrap_throw();
    }
}

fn remove_attribute(element: &web_sys::Element, name: &str) {
    element_attr_assertions(element, name);
    element.remove_attribute(name).unwrap_throw();
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
        Attributes::push(&mut element.modifier(), self.modifier.clone());
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
            Attributes::update(&mut element.modifier(), &prev.modifier, &self.modifier);
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
