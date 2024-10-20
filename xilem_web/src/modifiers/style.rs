// Copyright 2024 the Xilem Authors
// SPDX-License-Identifier: Apache-2.0

use crate::{
    core::{MessageResult, Mut, View, ViewElement, ViewId, ViewMarker},
    diff::{diff_iters, Diff},
    modifiers::With,
    vecmap::VecMap,
    DomView, DynMessage, ElementProps, ViewCtx,
};
use peniko::kurbo::Vec2;
use std::{
    collections::{BTreeMap, HashMap},
    fmt::{Debug, Display},
    hash::{BuildHasher, Hash},
    marker::PhantomData,
};
use wasm_bindgen::{JsCast, UnwrapThrowExt};

type CowStr = std::borrow::Cow<'static, str>;

#[derive(Debug, PartialEq, Clone)]
/// An modifier element to either set or remove an inline style.
///
/// It's used in [`Styles`].
pub enum StyleModifier {
    Set(CowStr, CowStr),
    Remove(CowStr),
}

impl StyleModifier {
    /// Returns the property name of this modifier.
    pub fn name(&self) -> &CowStr {
        let (StyleModifier::Set(name, _) | StyleModifier::Remove(name)) = self;
        name
    }

    /// Convert this modifier into its property name.
    pub fn into_name(self) -> CowStr {
        let (StyleModifier::Set(name, _) | StyleModifier::Remove(name)) = self;
        name
    }
}

impl<V: Into<Option<CowStr>>, K: Into<CowStr>> From<(K, V)> for StyleModifier {
    fn from((name, value): (K, V)) -> Self {
        match value.into() {
            Some(value) => StyleModifier::Set(name.into(), value),
            None => StyleModifier::Remove(name.into()),
        }
    }
}

/// A trait to make the style adding functions generic over collection types
pub trait StyleIter: PartialEq + Debug + 'static {
    // TODO do a similar pattern as in ClassIter? (i.e. don't use an Option here, and be able to use it as boolean intersection?)
    /// Iterates over key value pairs of style properties, `None` as value means remove the current value if it was previously set.
    fn styles_iter(&self) -> impl Iterator<Item = (CowStr, Option<CowStr>)>;

    fn style_modifiers_iter(&self) -> impl Iterator<Item = StyleModifier> {
        self.styles_iter().map(From::from)
    }
}

#[derive(PartialEq, Debug)]
struct StyleTuple<T1, T2>(T1, T2);

// TODO should this also allow removing style values, via `None`?
/// Create a style from a style name and its value.
pub fn style(
    name: impl Into<CowStr> + Clone + PartialEq + Debug + 'static,
    value: impl Into<CowStr> + Clone + PartialEq + Debug + 'static,
) -> impl StyleIter {
    StyleTuple(name, Some(value.into()))
}

impl<T1, T2> StyleIter for StyleTuple<T1, T2>
where
    T1: Into<CowStr> + Clone + PartialEq + Debug + 'static,
    T2: Into<Option<CowStr>> + Clone + PartialEq + Debug + 'static,
{
    fn styles_iter(&self) -> impl Iterator<Item = (CowStr, Option<CowStr>)> {
        let StyleTuple(key, value) = self;
        std::iter::once((key.clone().into(), value.clone().into()))
    }
}

impl<T: StyleIter> StyleIter for Option<T> {
    fn styles_iter(&self) -> impl Iterator<Item = (CowStr, Option<CowStr>)> {
        self.iter().flat_map(|c| c.styles_iter())
    }
}

impl<T: StyleIter> StyleIter for Vec<T> {
    fn styles_iter(&self) -> impl Iterator<Item = (CowStr, Option<CowStr>)> {
        self.iter().flat_map(|c| c.styles_iter())
    }
}

impl<T: StyleIter, const N: usize> StyleIter for [T; N] {
    fn styles_iter(&self) -> impl Iterator<Item = (CowStr, Option<CowStr>)> {
        self.iter().flat_map(|c| c.styles_iter())
    }
}

impl<T1, T2, S> StyleIter for HashMap<T1, T2, S>
where
    T1: Into<CowStr> + Clone + PartialEq + Eq + Hash + Debug + 'static,
    T2: Into<Option<CowStr>> + Clone + PartialEq + Debug + 'static,
    S: BuildHasher + 'static,
{
    fn styles_iter(&self) -> impl Iterator<Item = (CowStr, Option<CowStr>)> {
        self.iter()
            .map(|s| (s.0.clone().into(), s.1.clone().into()))
    }
}

impl<T1, T2> StyleIter for BTreeMap<T1, T2>
where
    T1: Into<CowStr> + Clone + PartialEq + Debug + 'static,
    T2: Into<Option<CowStr>> + Clone + PartialEq + Debug + 'static,
{
    fn styles_iter(&self) -> impl Iterator<Item = (CowStr, Option<CowStr>)> {
        self.iter()
            .map(|s| (s.0.clone().into(), s.1.clone().into()))
    }
}

impl<T1, T2> StyleIter for VecMap<T1, T2>
where
    T1: Into<CowStr> + Clone + PartialEq + Debug + 'static,
    T2: Into<Option<CowStr>> + Clone + PartialEq + Debug + 'static,
{
    fn styles_iter(&self) -> impl Iterator<Item = (CowStr, Option<CowStr>)> {
        self.iter()
            .map(|s| (s.0.clone().into(), s.1.clone().into()))
    }
}

#[derive(Default)]
/// An Element modifier that manages all inline styles of an Element.
pub struct Styles {
    // TODO think about using a `VecSplice` for more efficient insertion etc.,
    // but this is an additional trade-off of memory-usage and complexity,
    // while probably not helping much in the average case (of very few styles)...
    modifiers: Vec<StyleModifier>,
    updated: VecMap<CowStr, ()>,
    idx: u16,
    in_hydration: bool,
    was_created: bool,
}

impl With<Styles> for ElementProps {
    fn modifier(&mut self) -> &mut Styles {
        self.styles()
    }
}

fn set_style(element: &web_sys::Element, name: &str, value: &str) {
    if let Some(el) = element.dyn_ref::<web_sys::HtmlElement>() {
        el.style().set_property(name, value).unwrap_throw();
    } else if let Some(el) = element.dyn_ref::<web_sys::SvgElement>() {
        el.style().set_property(name, value).unwrap_throw();
    }
}

fn remove_style(element: &web_sys::Element, name: &str) {
    if let Some(el) = element.dyn_ref::<web_sys::HtmlElement>() {
        el.style().remove_property(name).unwrap_throw();
    } else if let Some(el) = element.dyn_ref::<web_sys::SvgElement>() {
        el.style().remove_property(name).unwrap_throw();
    }
}

impl Styles {
    /// Creates a new `Styles` modifier.
    ///
    /// `size_hint` is used to avoid unnecessary allocations while traversing up the view-tree when adding modifiers in [`View::build`].
    pub(crate) fn new(size_hint: usize, in_hydration: bool) -> Self {
        Self {
            modifiers: Vec::with_capacity(size_hint),
            was_created: true,
            in_hydration,
            ..Default::default()
        }
    }

    /// Applies potential changes of the inline styles of an element to the underlying DOM node.
    pub fn apply_changes(&mut self, element: &web_sys::Element) {
        if self.in_hydration {
            self.in_hydration = false;
            self.was_created = false;
        } else if self.was_created {
            self.was_created = false;
            for modifier in &self.modifiers {
                match modifier {
                    StyleModifier::Remove(name) => remove_style(element, name),
                    StyleModifier::Set(name, value) => set_style(element, name, value),
                }
            }
        } else if !self.updated.is_empty() {
            for modifier in self.modifiers.iter().rev() {
                match modifier {
                    StyleModifier::Remove(name) if self.updated.remove(name).is_some() => {
                        remove_style(element, name);
                    }
                    StyleModifier::Set(name, value) if self.updated.remove(name).is_some() => {
                        set_style(element, name, value);
                    }
                    _ => {}
                }
            }
            // if there's any remaining key in updated, it means these are deleted keys
            for (name, ()) in self.updated.drain() {
                remove_style(element, &name);
            }
        }
        debug_assert!(self.updated.is_empty());
    }

    /// Returns a previous [`StyleModifier`], when `predicate` returns true, this is similar to [`Iterator::find`].
    pub fn get(&self, mut predicate: impl FnMut(&StyleModifier) -> bool) -> Option<&StyleModifier> {
        self.modifiers[..self.idx as usize]
            .iter()
            .rev()
            .find(|modifier| predicate(modifier))
    }

    #[inline]
    /// Returns the current value of a style property with `name` if it is set.
    pub fn get_style(&self, name: &str) -> Option<&CowStr> {
        if let Some(StyleModifier::Set(_, value)) = self.get(
            |m| matches!(m, StyleModifier::Remove(key) | StyleModifier::Set(key, _) if key == name),
        ) {
            Some(value)
        } else {
            None
        }
    }

    #[inline]
    /// Rebuilds the current element, while ensuring that the order of the modifiers stays correct.
    /// Any children should be rebuilt in inside `f`, *before* modifing any other properties of [`Styles`].
    pub fn rebuild<E: With<Self>>(mut element: E, prev_len: usize, f: impl FnOnce(E)) {
        element.modifier().idx -= prev_len as u16;
        f(element);
    }

    #[inline]
    /// Returns whether the underlying element has been built or rebuilt, this could e.g. happen, when `OneOf` changes a variant to a different element.
    pub fn was_created(&self) -> bool {
        self.was_created
    }

    #[inline]
    /// Returns whether the style with the `name` has been modified in the current reconciliation pass/rebuild.
    fn was_updated(&self, name: &str) -> bool {
        self.updated.contains_key(name)
    }

    #[inline]
    /// Pushes `modifier` at the end of the current modifiers
    ///
    /// Must only be used when `self.was_created() == true`, use `Styles::insert` otherwise.
    pub fn push(&mut self, modifier: StyleModifier) {
        debug_assert!(
            self.was_created(),
            "This should never be called, when the underlying element wasn't (re)created. Use `Styles::insert` instead."
        );
        if !self.was_created && !self.in_hydration {
            self.updated.insert(modifier.name().clone(), ());
        }
        self.modifiers.push(modifier);
        self.idx += 1;
    }

    #[inline]
    /// Inserts `modifier` at the current index
    ///
    /// Must only be used when `self.was_created() == false`, use `Styles::push` otherwise.
    pub fn insert(&mut self, modifier: StyleModifier) {
        debug_assert!(
            !self.was_created(),
            "This should never be called, when the underlying element was (re)created, use `Styles::push` instead."
        );
        if !self.was_created && !self.in_hydration {
            self.updated.insert(modifier.name().clone(), ());
        }
        // TODO this could potentially be expensive, maybe think about `VecSplice` again.
        // Although in the average case, this is likely not relevant, as usually very few attributes are used, thus shifting is probably good enough
        // I.e. a `VecSplice` is probably less optimal (either more complicated code, and/or more memory usage)
        self.modifiers.insert(self.idx as usize, modifier);
        self.idx += 1;
    }

    #[inline]
    /// Mutates the next modifier.
    ///
    /// Must only be used when `self.was_created() == false`.
    pub fn mutate<R>(&mut self, f: impl FnOnce(&mut StyleModifier) -> R) -> R {
        debug_assert!(
            !self.was_created(),
            "This should never be called, when the underlying element was (re)created."
        );
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

    #[inline]
    /// Skips the next `count` modifiers.
    ///
    /// Must only be used when `self.was_created() == false`.
    pub fn skip(&mut self, count: usize) {
        debug_assert!(
            !self.was_created(),
            "This should never be called, when the underlying element was (re)created."
        );
        self.idx += count as u16;
    }

    #[inline]
    /// Deletes the next `count` modifiers.
    ///
    /// Must only be used when `self.was_created() == false`.
    pub fn delete(&mut self, count: usize) {
        debug_assert!(
            !self.was_created(),
            "This should never be called, when the underlying element was (re)created."
        );
        let start = self.idx as usize;
        for modifier in self.modifiers.drain(start..(start + count)) {
            self.updated.insert(modifier.into_name(), ());
        }
    }

    #[inline]
    /// Updates the next modifier, based on the diff of `prev` and `next`.
    pub fn update(&mut self, prev: &StyleModifier, next: &StyleModifier) {
        if self.was_created() {
            self.push(next.clone());
        } else if next != prev {
            self.mutate(|modifier| *modifier = next.clone());
        } else {
            self.skip(1);
        }
    }

    #[inline]
    /// Extends the current modifiers with an iterator of modifiers. Returns the count of `modifiers`.
    ///
    /// Must only be used when `self.was_created() == true`, use `Styles::apply_diff` otherwise.
    pub fn extend(&mut self, modifiers: impl Iterator<Item = StyleModifier>) -> usize {
        debug_assert!(
            self.was_created(),
            "This should never be called, when the underlying element wasn't (re)created, use `Styles::apply_diff` instead."
        );
        let prev_len = self.modifiers.len();
        self.modifiers.extend(modifiers);
        let iter_count = self.modifiers.len() - prev_len;
        if !self.was_created && !self.in_hydration && iter_count > 0 {
            for modifier in &self.modifiers[prev_len..] {
                self.updated.insert(modifier.name().clone(), ());
            }
        }
        self.idx += iter_count as u16;
        iter_count
    }

    #[inline]
    /// Diffs between two iterators, and updates the underlying modifiers if they have changed, returns the `next` iterator count.
    ///
    /// Must only be used when `self.was_created() == false`, use [`Styles::extend`] otherwise.
    pub fn apply_diff<T: Iterator<Item = StyleModifier>>(&mut self, prev: T, next: T) -> usize {
        debug_assert!(
            !self.was_created(),
            "This should never be called, when the underlying element was (re)created, use `Styles::extend` instead."
        );
        let mut count = 0;
        for change in diff_iters(prev, next) {
            match change {
                Diff::Add(modifier) => {
                    self.insert(modifier);
                    count += 1;
                }
                Diff::Remove(count) => self.delete(count),
                Diff::Change(new_modifier) => {
                    self.mutate(|modifier| *modifier = new_modifier);
                    count += 1;
                }
                Diff::Skip(c) => {
                    self.skip(c);
                    count += c;
                }
            }
        }
        count
    }

    #[inline]
    /// Updates styles defined by an iterator, returns the `next` iterator length.
    pub fn update_style_modifier_iter<T: StyleIter>(
        &mut self,
        prev_len: usize,
        prev: &T,
        next: &T,
    ) -> usize {
        if self.was_created() {
            self.extend(next.style_modifiers_iter())
        } else if next != prev {
            self.apply_diff(prev.style_modifiers_iter(), next.style_modifiers_iter())
        } else {
            self.skip(prev_len);
            prev_len
        }
    }

    #[inline]
    /// Updates the style property `name` by modifying its previous value with `create_modifier`.
    pub fn update_style_mutator<T: PartialEq>(
        &mut self,
        name: &'static str,
        prev: &T,
        next: &T,
        create_modifier: impl FnOnce(Option<&CowStr>, &T) -> StyleModifier,
    ) {
        if self.was_created() {
            self.push(create_modifier(self.get_style(name), next));
        } else if prev != next || self.was_updated(name) {
            let new_modifier = create_modifier(self.get_style(name), next);
            self.mutate(|modifier| *modifier = new_modifier);
        } else {
            self.skip(1);
        }
    }
}

#[derive(Clone, Debug)]
/// A view to add `style` properties to `Element` derived elements.
///
/// See [`Element::style`](`crate::interfaces::Element::style`) for more usage information.
pub struct Style<E, S, T, A> {
    el: E,
    styles: S,
    phantom: PhantomData<fn() -> (T, A)>,
}

impl<E, S, T, A> Style<E, S, T, A> {
    /// Create a `Style` view. `styles` is a [`StyleIter`].
    ///
    /// Usually [`Element::style`](`crate::interfaces::Element::style`) should be used instead of this function.
    pub fn new(el: E, styles: S) -> Self {
        Style {
            el,
            styles,
            phantom: PhantomData,
        }
    }
}

impl<E, S, State, Action> ViewMarker for Style<E, S, State, Action> {}
impl<V, S, State, Action> View<State, Action, ViewCtx, DynMessage> for Style<V, S, State, Action>
where
    State: 'static,
    Action: 'static,
    S: StyleIter,
    V: DomView<State, Action, Element: With<Styles>>,
    for<'a> <V::Element as ViewElement>::Mut<'a>: With<Styles>,
{
    type Element = V::Element;

    type ViewState = (usize, V::ViewState);

    fn build(&self, ctx: &mut ViewCtx) -> (Self::Element, Self::ViewState) {
        let style_iter = self.styles.style_modifiers_iter();
        let (mut e, s) =
            ctx.with_size_hint::<Styles, _>(style_iter.size_hint().0, |ctx| self.el.build(ctx));
        let len = e.modifier().extend(style_iter);
        (e, (len, s))
    }

    fn rebuild(
        &self,
        prev: &Self,
        (len, view_state): &mut Self::ViewState,
        ctx: &mut ViewCtx,
        element: Mut<Self::Element>,
    ) {
        Styles::rebuild(element, *len, |mut elem| {
            self.el
                .rebuild(&prev.el, view_state, ctx, elem.reborrow_mut());
            let styles = elem.modifier();
            *len = styles.update_style_modifier_iter(*len, &prev.styles, &self.styles);
        });
    }

    fn teardown(
        &self,
        (_, view_state): &mut Self::ViewState,
        ctx: &mut ViewCtx,
        element: Mut<Self::Element>,
    ) {
        self.el.teardown(view_state, ctx, element);
    }

    fn message(
        &self,
        (_, view_state): &mut Self::ViewState,
        id_path: &[ViewId],
        message: DynMessage,
        app_state: &mut State,
    ) -> MessageResult<Action, DynMessage> {
        self.el.message(view_state, id_path, message, app_state)
    }
}

/// Add a `rotate(<radians>rad)` [transform-function](https://developer.mozilla.org/en-US/docs/Web/CSS/transform-function/rotate) to the current CSS `transform`.
pub struct Rotate<E, State, Action> {
    el: E,
    phantom: PhantomData<fn() -> (State, Action)>,
    radians: f64,
}

impl<E, State, Action> Rotate<E, State, Action> {
    pub(crate) fn new(element: E, radians: f64) -> Self {
        Rotate {
            el: element,
            phantom: PhantomData,
            radians,
        }
    }
}

fn rotate_transform_modifier(transform: Option<&CowStr>, radians: &f64) -> StyleModifier {
    let value = if let Some(transform) = transform {
        format!("{transform} rotate({radians}rad)")
    } else {
        format!("rotate({radians}rad)")
    };
    StyleModifier::Set("transform".into(), CowStr::from(value))
}

impl<V, State, Action> ViewMarker for Rotate<V, State, Action> {}
impl<V, State, Action> View<State, Action, ViewCtx, DynMessage> for Rotate<V, State, Action>
where
    State: 'static,
    Action: 'static,
    V: DomView<State, Action, Element: With<Styles>>,
    for<'a> <V::Element as ViewElement>::Mut<'a>: With<Styles>,
{
    type Element = V::Element;

    type ViewState = V::ViewState;

    fn build(&self, ctx: &mut ViewCtx) -> (Self::Element, Self::ViewState) {
        let (mut element, state) = ctx.with_size_hint::<Styles, _>(1, |ctx| self.el.build(ctx));
        let styles = element.modifier();
        styles.push(rotate_transform_modifier(
            styles.get_style("transform"),
            &self.radians,
        ));
        (element, state)
    }

    fn rebuild(
        &self,
        prev: &Self,
        view_state: &mut Self::ViewState,
        ctx: &mut ViewCtx,
        element: Mut<Self::Element>,
    ) {
        Styles::rebuild(element, 1, |mut element| {
            self.el
                .rebuild(&prev.el, view_state, ctx, element.reborrow_mut());
            element.modifier().update_style_mutator(
                "transform",
                &prev.radians,
                &self.radians,
                rotate_transform_modifier,
            );
        });
    }

    fn teardown(
        &self,
        view_state: &mut Self::ViewState,
        ctx: &mut ViewCtx,
        element: Mut<Self::Element>,
    ) {
        self.el.teardown(view_state, ctx, element);
    }

    fn message(
        &self,
        view_state: &mut Self::ViewState,
        id_path: &[ViewId],
        message: DynMessage,
        app_state: &mut State,
    ) -> MessageResult<Action, DynMessage> {
        self.el.message(view_state, id_path, message, app_state)
    }
}

#[derive(Clone, Copy, Debug, PartialEq)]
/// A wrapper, for some syntax sugar, such that `el.scale(1.5).scale((0.75, 2.0))` is possible.
pub enum ScaleValue {
    Uniform(f64),
    NonUniform(f64, f64),
}

impl Display for ScaleValue {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            ScaleValue::Uniform(uniform) => write!(f, "{uniform}"),
            ScaleValue::NonUniform(x, y) => write!(f, "{x}, {y}"),
        }
    }
}

impl From<f64> for ScaleValue {
    fn from(value: f64) -> Self {
        ScaleValue::Uniform(value)
    }
}

impl From<(f64, f64)> for ScaleValue {
    fn from(value: (f64, f64)) -> Self {
        ScaleValue::NonUniform(value.0, value.1)
    }
}

impl From<Vec2> for ScaleValue {
    fn from(value: Vec2) -> Self {
        ScaleValue::NonUniform(value.x, value.y)
    }
}

/// Add a `scale(<scale>)` [transform-function](https://developer.mozilla.org/en-US/docs/Web/CSS/transform-function/scale) to the current CSS `transform`.
pub struct Scale<E, State, Action> {
    el: E,
    phantom: PhantomData<fn() -> (State, Action)>,
    scale: ScaleValue,
}

impl<E, State, Action> Scale<E, State, Action> {
    pub(crate) fn new(element: E, scale: impl Into<ScaleValue>) -> Self {
        Scale {
            el: element,
            phantom: PhantomData,
            scale: scale.into(),
        }
    }
}

fn scale_transform_modifier(transform: Option<&CowStr>, scale: &ScaleValue) -> StyleModifier {
    let value = if let Some(transform) = transform {
        format!("{transform} scale({scale})")
    } else {
        format!("scale({scale})")
    };
    StyleModifier::Set("transform".into(), CowStr::from(value))
}

impl<E, State, Action> ViewMarker for Scale<E, State, Action> {}
impl<State, Action, V> View<State, Action, ViewCtx, DynMessage> for Scale<V, State, Action>
where
    State: 'static,
    Action: 'static,
    V: DomView<State, Action, Element: With<Styles>>,
    for<'a> <V::Element as ViewElement>::Mut<'a>: With<Styles>,
{
    type Element = V::Element;

    type ViewState = V::ViewState;

    fn build(&self, ctx: &mut ViewCtx) -> (Self::Element, Self::ViewState) {
        let (mut element, state) = ctx.with_size_hint::<Styles, _>(1, |ctx| self.el.build(ctx));
        let styles = element.modifier();
        styles.push(scale_transform_modifier(
            styles.get_style("transform"),
            &self.scale,
        ));
        (element, state)
    }

    fn rebuild(
        &self,
        prev: &Self,
        view_state: &mut Self::ViewState,
        ctx: &mut ViewCtx,
        element: Mut<Self::Element>,
    ) {
        Styles::rebuild(element, 1, |mut element| {
            self.el
                .rebuild(&prev.el, view_state, ctx, element.reborrow_mut());
            element.modifier().update_style_mutator(
                "transform",
                &prev.scale,
                &self.scale,
                scale_transform_modifier,
            );
        });
    }

    fn teardown(
        &self,
        view_state: &mut Self::ViewState,
        ctx: &mut ViewCtx,
        element: Mut<Self::Element>,
    ) {
        self.el.teardown(view_state, ctx, element);
    }

    fn message(
        &self,
        view_state: &mut Self::ViewState,
        id_path: &[ViewId],
        message: DynMessage,
        app_state: &mut State,
    ) -> MessageResult<Action, DynMessage> {
        self.el.message(view_state, id_path, message, app_state)
    }
}
