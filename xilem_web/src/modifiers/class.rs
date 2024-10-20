// Copyright 2024 the Xilem Authors
// SPDX-License-Identifier: Apache-2.0

use crate::{
    core::{MessageResult, Mut, View, ViewElement, ViewId, ViewMarker},
    diff::{diff_iters, Diff},
    vecmap::VecMap,
    DomView, DynMessage, ElementProps, ViewCtx, With,
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

const IN_HYDRATION: u8 = 1 << 0;
const WAS_CREATED: u8 = 1 << 1;

#[derive(Default)]
/// An Element modifier that manages all classes of an Element.
pub struct Classes {
    class_name: String,
    classes: VecMap<CowStr, ()>,
    modifiers: Vec<ClassModifier>,
    idx: u16,
    dirty: bool,
    /// This is to avoid an additional alignment word with 2 booleans, it contains the two `IN_HYDRATION` and `WAS_CREATED` flags
    flags: u8,
}

impl With<Classes> for ElementProps {
    fn modifier(&mut self) -> &mut Classes {
        self.classes()
    }
}

impl Classes {
    /// Creates a new `Classes` modifier.
    ///
    /// `size_hint` is used to avoid unnecessary allocations while traversing up the view-tree when adding modifiers in [`View::build`].
    pub(crate) fn new(size_hint: usize, in_hydration: bool) -> Self {
        let mut flags = WAS_CREATED;
        if in_hydration {
            flags |= IN_HYDRATION;
        }
        Self {
            modifiers: Vec::with_capacity(size_hint),
            flags,
            ..Default::default()
        }
    }

    /// Applies potential changes of the classes of an element to the underlying DOM node.
    pub fn apply_changes(&mut self, element: &web_sys::Element) {
        if (self.flags & IN_HYDRATION) == IN_HYDRATION {
            self.flags = 0;
            self.dirty = false;
        } else if self.dirty {
            self.flags = 0;
            self.dirty = false;
            self.classes.clear();
            self.classes.reserve(self.modifiers.len());
            for modifier in &self.modifiers {
                match modifier {
                    ClassModifier::Remove(class_name) => self.classes.remove(class_name),
                    ClassModifier::Add(class_name) => self.classes.insert(class_name.clone(), ()),
                };
            }
            self.class_name.clear();
            self.class_name
                .reserve_exact(self.classes.keys().map(|k| k.len() + 1).sum());
            let last_idx = self.classes.len().saturating_sub(1);
            for (idx, class) in self.classes.keys().enumerate() {
                self.class_name += class;
                if idx != last_idx {
                    self.class_name += " ";
                }
            }
            // Svg elements do have issues with className, see https://developer.mozilla.org/en-US/docs/Web/API/Element/className
            if element.dyn_ref::<web_sys::SvgElement>().is_some() {
                element
                    .set_attribute(wasm_bindgen::intern("class"), &self.class_name)
                    .unwrap_throw();
            } else {
                element.set_class_name(&self.class_name);
            }
        }
    }

    #[inline]
    /// Rebuilds the current element, while ensuring that the order of the modifiers stays correct.
    /// Any children should be rebuilt in inside `f`, *before* modifing any other properties of [`Classes`].
    pub fn rebuild<E: With<Self>>(mut element: E, prev_len: usize, f: impl FnOnce(E)) {
        element.modifier().idx -= prev_len as u16;
        f(element);
    }

    #[inline]
    /// Returns whether the underlying element has been rebuilt, this could e.g. happen, when `OneOf` changes a variant to a different element.
    pub fn was_created(&self) -> bool {
        self.flags & WAS_CREATED != 0
    }

    #[inline]
    /// Pushes `modifier` at the end of the current modifiers.
    ///
    /// Must only be used when `self.was_created() == true`
    pub fn push(&mut self, modifier: ClassModifier) {
        debug_assert!(
            self.was_created(),
            "This should never be called, when the underlying element wasn't (re)created, use `Classes::push` instead"
        );
        self.dirty = true;
        self.modifiers.push(modifier);
        self.idx += 1;
    }

    #[inline]
    /// Inserts `modifier` at the current index.
    ///
    /// Must only be used when `self.was_created() == false`
    pub fn insert(&mut self, modifier: ClassModifier) {
        debug_assert!(
            !self.was_created(),
            "This should never be called, when the underlying element was (re)created, use `Classes::push` instead"
        );

        self.dirty = true;
        // TODO this could potentially be expensive, maybe think about `VecSplice` again.
        // Although in the average case, this is likely not relevant, as usually very few attributes are used, thus shifting is probably good enough
        // I.e. a `VecSplice` is probably less optimal (either more complicated code, and/or more memory usage)
        self.modifiers.insert(self.idx as usize, modifier);
        self.idx += 1;
    }

    #[inline]
    /// Mutates the next modifier.
    ///
    /// Must only be used when `!self.was_created()`
    pub fn mutate<R>(&mut self, f: impl FnOnce(&mut ClassModifier) -> R) -> R {
        debug_assert!(
            !self.was_created(),
            "This should never be called, when the underlying element was (re)created, use `Classes::push` instead"
        );

        self.dirty = true;
        let idx = self.idx;
        self.idx += 1;
        f(&mut self.modifiers[idx as usize])
    }

    #[inline]
    /// Skips the next `count` modifiers.
    pub fn skip(&mut self, count: usize) {
        self.idx += count as u16;
    }

    #[inline]
    /// Deletes the next `count` modifiers.
    pub fn delete(&mut self, count: usize) {
        let start = self.idx as usize;
        self.dirty = true;
        self.modifiers.drain(start..(start + count));
    }

    #[inline]
    /// Extends the current modifiers with an iterator of modifiers. Returns the count of `modifiers`.
    pub fn extend(&mut self, modifiers: impl Iterator<Item = ClassModifier>) -> usize {
        self.dirty = true;
        let prev_len = self.modifiers.len();
        self.modifiers.extend(modifiers);
        let new_len = self.modifiers.len() - prev_len;
        self.idx += new_len as u16;
        new_len
    }

    #[inline]
    /// Diffs between two iterators, and updates the underlying modifiers if they have changed, returns the `next` iterator count.
    pub fn apply_diff<T: Iterator<Item = ClassModifier>>(&mut self, prev: T, next: T) -> usize {
        let mut new_len = 0;
        for change in diff_iters(prev, next) {
            match change {
                Diff::Add(modifier) => {
                    self.insert(modifier);
                    new_len += 1;
                }
                Diff::Remove(count) => self.delete(count),
                Diff::Change(new_modifier) => {
                    self.mutate(|modifier| *modifier = new_modifier);
                    new_len += 1;
                }
                Diff::Skip(count) => {
                    self.skip(count);
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
        &mut self,
        prev_len: usize,
        prev: &T,
        next: &T,
    ) -> usize {
        if self.was_created() {
            self.extend(next.add_class_iter())
        } else if next != prev {
            self.apply_diff(prev.add_class_iter(), next.add_class_iter())
        } else {
            self.skip(prev_len);
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
    V: DomView<State, Action, Element: With<Classes>>,
    for<'a> <V::Element as ViewElement>::Mut<'a>: With<Classes>,
{
    type Element = V::Element;

    type ViewState = (usize, V::ViewState);

    fn build(&self, ctx: &mut ViewCtx) -> (Self::Element, Self::ViewState) {
        let add_class_iter = self.classes.add_class_iter();
        let (mut e, s) = ctx
            .with_size_hint::<Classes, _>(add_class_iter.size_hint().0, |ctx| self.el.build(ctx));
        let len = e.modifier().extend(add_class_iter);
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
            let classes = elem.modifier();
            *len = classes.update_as_add_class_iter(*len, &prev.classes, &self.classes);
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
