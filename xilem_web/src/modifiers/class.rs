// Copyright 2024 the Xilem Authors
// SPDX-License-Identifier: Apache-2.0

use crate::{
    core::{MessageResult, Mut, View, ViewElement, ViewId, ViewMarker},
    diff::{diff_iters, Diff},
    modifiers::{Modifier, WithModifier},
    vecmap::VecMap,
    DomView, DynMessage, ViewCtx,
};
use std::{fmt::Debug, marker::PhantomData};
use wasm_bindgen::{JsCast, UnwrapThrowExt};

type CowStr = std::borrow::Cow<'static, str>;

#[derive(Debug, PartialEq, Clone)]
/// An modifier element to either add or remove a class of an element.
///
/// It's used in [`Classes`].
pub enum ClassModifier {
    Add(CowStr),
    Remove(CowStr),
}

impl ClassModifier {
    /// Returns the class name of this modifier.
    pub fn name(&self) -> &CowStr {
        let (ClassModifier::Add(name) | ClassModifier::Remove(name)) = self;
        name
    }
}

/// Types implementing this trait can be used in the [`Class`] view, see also [`Element::class`](`crate::interfaces::Element::class`).
pub trait ClassIter: PartialEq + Debug + 'static {
    /// Returns an iterator of class compliant strings (e.g. the strings aren't allowed to contain spaces).
    fn class_iter(&self) -> impl Iterator<Item = CowStr>;

    /// Returns an iterator of additive classes, i.e. all classes of this iterator are added to the current element.
    fn add_class_iter(&self) -> impl Iterator<Item = ClassModifier> {
        self.class_iter().map(ClassModifier::Add)
    }

    /// Returns an iterator of to remove classes, i.e. all classes of this iterator are removed from the current element.
    fn remove_class_iter(&self) -> impl Iterator<Item = ClassModifier> {
        self.class_iter().map(ClassModifier::Remove)
    }
}

impl<C: ClassIter> ClassIter for Option<C> {
    fn class_iter(&self) -> impl Iterator<Item = CowStr> {
        self.iter().flat_map(|c| c.class_iter())
    }
}

impl ClassIter for String {
    fn class_iter(&self) -> impl Iterator<Item = CowStr> {
        std::iter::once(self.clone().into())
    }
}

impl ClassIter for &'static str {
    fn class_iter(&self) -> impl Iterator<Item = CowStr> {
        std::iter::once(CowStr::from(*self))
    }
}

impl ClassIter for CowStr {
    fn class_iter(&self) -> impl Iterator<Item = CowStr> {
        std::iter::once(self.clone())
    }
}

impl<C: ClassIter> ClassIter for Vec<C> {
    fn class_iter(&self) -> impl Iterator<Item = CowStr> {
        self.iter().flat_map(|c| c.class_iter())
    }
}

impl<C: ClassIter, const N: usize> ClassIter for [C; N] {
    fn class_iter(&self) -> impl Iterator<Item = CowStr> {
        self.iter().flat_map(|c| c.class_iter())
    }
}

#[derive(Default)]
/// An Element modifier that manages all classes of an Element.
pub struct Classes {
    class_name: String,
    // It would be nice to avoid this, as this results in extra allocations, when `Strings` are used as classes.
    classes: VecMap<CowStr, ()>,
    modifiers: Vec<ClassModifier>,
    idx: u16,
    dirty: bool,
}

impl Classes {
    /// Creates a new `Classes` modifier.
    ///
    /// `size_hint` is used to avoid unnecessary allocations while traversing up the view-tree when adding modifiers in [`View::build`].
    pub(crate) fn new(size_hint: usize) -> Self {
        Self {
            modifiers: Vec::with_capacity(size_hint),
            ..Default::default()
        }
    }

    /// Applies potential changes of the classes of an element to the underlying DOM node.
    pub fn apply_changes(this: Modifier<'_, Self>, element: &web_sys::Element) {
        let Modifier { modifier, flags } = this;

        if flags.in_hydration() {
            modifier.dirty = false;
        } else if modifier.dirty {
            modifier.dirty = false;
            modifier.classes.clear();
            modifier.classes.reserve(modifier.modifiers.len());
            for m in &modifier.modifiers {
                match m {
                    ClassModifier::Remove(class_name) => modifier.classes.remove(class_name),
                    ClassModifier::Add(class_name) => {
                        modifier.classes.insert(class_name.clone(), ())
                    }
                };
            }
            modifier.class_name.clear();
            modifier
                .class_name
                .reserve_exact(modifier.classes.keys().map(|k| k.len() + 1).sum());
            let last_idx = modifier.classes.len().saturating_sub(1);
            for (idx, class) in modifier.classes.keys().enumerate() {
                modifier.class_name += class;
                if idx != last_idx {
                    modifier.class_name += " ";
                }
            }
            // Svg elements do have issues with className, see https://developer.mozilla.org/en-US/docs/Web/API/Element/className
            if element.dyn_ref::<web_sys::SvgElement>().is_some() {
                element
                    .set_attribute(wasm_bindgen::intern("class"), &modifier.class_name)
                    .unwrap_throw();
            } else {
                element.set_class_name(&modifier.class_name);
            }
        }
    }

    #[inline]
    /// Rebuilds the current element, while ensuring that the order of the modifiers stays correct.
    /// Any children should be rebuilt in inside `f`, *before* modifying any other properties of [`Classes`].
    pub fn rebuild<E: WithModifier<Self>>(mut element: E, prev_len: usize, f: impl FnOnce(E)) {
        element.modifier().modifier.idx -= prev_len as u16;
        f(element);
    }

    #[inline]
    /// Pushes `modifier` at the end of the current modifiers.
    ///
    /// Must only be used when `this.flags.was_created() == true`
    pub fn push(this: &mut Modifier<'_, Self>, modifier: ClassModifier) {
        debug_assert!(
            this.flags.was_created(),
            "This should never be called, when the underlying element wasn't (re)created. Use `Classes::insert` instead."
        );
        this.modifier.dirty = true;
        this.flags.set_needs_update();
        this.modifier.modifiers.push(modifier);
        this.modifier.idx += 1;
    }

    #[inline]
    /// Inserts `modifier` at the current index.
    ///
    /// Must only be used when `this.flags.was_created() == false`
    pub fn insert(this: &mut Modifier<'_, Self>, modifier: ClassModifier) {
        debug_assert!(
            !this.flags.was_created(),
            "This should never be called, when the underlying element was (re)created, use `Classes::push` instead."
        );

        this.modifier.dirty = true;
        this.flags.set_needs_update();
        // TODO this could potentially be expensive, maybe think about `VecSplice` again.
        // Although in the average case, this is likely not relevant, as usually very few attributes are used, thus shifting is probably good enough
        // I.e. a `VecSplice` is probably less optimal (either more complicated code, and/or more memory usage)
        this.modifier
            .modifiers
            .insert(this.modifier.idx as usize, modifier);
        this.modifier.idx += 1;
    }

    #[inline]
    /// Mutates the next modifier.
    ///
    /// Must only be used when `this.flags.was_created() == false`
    pub fn mutate<R>(this: &mut Modifier<'_, Self>, f: impl FnOnce(&mut ClassModifier) -> R) -> R {
        debug_assert!(
            !this.flags.was_created(),
            "This should never be called, when the underlying element was (re)created, use `Classes::push` instead."
        );

        this.modifier.dirty = true;
        this.flags.set_needs_update();
        let idx = this.modifier.idx;
        this.modifier.idx += 1;
        f(&mut this.modifier.modifiers[idx as usize])
    }

    #[inline]
    /// Skips the next `count` modifiers.
    ///
    /// Must only be used when `this.flags.was_created() == false`
    pub fn skip(this: &mut Modifier<'_, Self>, count: usize) {
        debug_assert!(
            !this.flags.was_created(),
            "This should never be called, when the underlying element was (re)created"
        );
        this.modifier.idx += count as u16;
    }

    #[inline]
    /// Deletes the next `count` modifiers.
    ///
    /// Must only be used when `this.flags.was_created() == false`
    pub fn delete(this: &mut Modifier<'_, Self>, count: usize) {
        debug_assert!(
            !this.flags.was_created(),
            "This should never be called, when the underlying element was (re)created."
        );
        let start = this.modifier.idx as usize;
        this.modifier.dirty = true;
        this.flags.set_needs_update();
        this.modifier.modifiers.drain(start..(start + count));
    }

    #[inline]
    /// Extends the current modifiers with an iterator of modifiers. Returns the count of `modifiers`.
    ///
    /// Must only be used when `this.flags.was_created() == true`
    pub fn extend(
        this: &mut Modifier<'_, Self>,
        modifiers: impl Iterator<Item = ClassModifier>,
    ) -> usize {
        debug_assert!(
            this.flags.was_created(),
            "This should never be called, when the underlying element wasn't (re)created, use `Classes::apply_diff` instead."
        );
        this.modifier.dirty = true;
        this.flags.set_needs_update();
        let prev_len = this.modifier.modifiers.len();
        this.modifier.modifiers.extend(modifiers);
        let new_len = this.modifier.modifiers.len() - prev_len;
        this.modifier.idx += new_len as u16;
        new_len
    }

    #[inline]
    /// Diffs between two iterators, and updates the underlying modifiers if they have changed, returns the `next` iterator count.
    ///
    /// Must only be used when `this.flags.was_created() == false`
    pub fn apply_diff<T: Iterator<Item = ClassModifier>>(
        this: &mut Modifier<'_, Self>,
        prev: T,
        next: T,
    ) -> usize {
        debug_assert!(
            !this.flags.was_created(),
            "This should never be called, when the underlying element was (re)created, use `Classes::extend` instead."
        );
        let mut new_len = 0;
        for change in diff_iters(prev, next) {
            match change {
                Diff::Add(modifier) => {
                    Classes::insert(this, modifier);
                    new_len += 1;
                }
                Diff::Remove(count) => Classes::delete(this, count),
                Diff::Change(new_modifier) => {
                    Classes::mutate(this, |modifier| *modifier = new_modifier);
                    new_len += 1;
                }
                Diff::Skip(count) => {
                    Classes::skip(this, count);
                    new_len += count;
                }
            }
        }
        new_len
    }

    /// Updates based on the diff between two class iterators (`prev`, `next`) interpreted as add modifiers.
    ///
    /// Updates the underlying modifiers if they have changed, returns the next iterator count.
    /// Skips or adds modifiers, when nothing has changed, or the element was recreated.
    pub fn update_as_add_class_iter<T: ClassIter>(
        this: &mut Modifier<'_, Self>,
        prev_len: usize,
        prev: &T,
        next: &T,
    ) -> usize {
        if this.flags.was_created() {
            Classes::extend(this, next.add_class_iter())
        } else if next != prev {
            Classes::apply_diff(this, prev.add_class_iter(), next.add_class_iter())
        } else {
            Classes::skip(this, prev_len);
            prev_len
        }
    }
}

/// A view to add classes to `Element` derived elements.
///
/// See [`Element::class`](`crate::interfaces::Element::class`) for more usage information.
#[derive(Clone, Debug)]
pub struct Class<E, C, T, A> {
    el: E,
    classes: C,
    phantom: PhantomData<fn() -> (T, A)>,
}

impl<E, C, T, A> Class<E, C, T, A> {
    /// Create a `Class` view. `classes` is a [`ClassIter`].
    ///
    /// Usually [`Element::class`](`crate::interfaces::Element::class`) should be used instead of this function.
    pub fn new(el: E, classes: C) -> Self {
        Class {
            el,
            classes,
            phantom: PhantomData,
        }
    }
}

impl<V, C, State, Action> ViewMarker for Class<V, C, State, Action> {}
impl<V, C, State, Action> View<State, Action, ViewCtx, DynMessage> for Class<V, C, State, Action>
where
    State: 'static,
    Action: 'static,
    C: ClassIter,
    V: DomView<State, Action, Element: WithModifier<Classes>>,
    for<'a> <V::Element as ViewElement>::Mut<'a>: WithModifier<Classes>,
{
    type Element = V::Element;

    type ViewState = (usize, V::ViewState);

    fn build(&self, ctx: &mut ViewCtx) -> (Self::Element, Self::ViewState) {
        let add_class_iter = self.classes.add_class_iter();
        let (mut e, s) = ctx
            .with_size_hint::<Classes, _>(add_class_iter.size_hint().0, |ctx| self.el.build(ctx));
        let len = Classes::extend(&mut e.modifier(), add_class_iter);
        (e, (len, s))
    }

    fn rebuild(
        &self,
        prev: &Self,
        (len, view_state): &mut Self::ViewState,
        ctx: &mut ViewCtx,
        element: Mut<Self::Element>,
    ) {
        Classes::rebuild(element, *len, |mut elem| {
            self.el
                .rebuild(&prev.el, view_state, ctx, elem.reborrow_mut());
            *len = Classes::update_as_add_class_iter(
                &mut elem.modifier(),
                *len,
                &prev.classes,
                &self.classes,
            );
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
