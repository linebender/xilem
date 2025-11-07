// Copyright 2025 the Xilem Authors
// SPDX-License-Identifier: Apache-2.0

use alloc::vec::{Drain, Vec};

use crate::ViewElement;

/// A temporary "splice" to add, update and delete in an (ordered) sequence of elements.
/// It is mainly intended for view sequences.
pub trait ElementSplice<Element: ViewElement> {
    /// Run a function with access to the associated [`AppendVec`].
    ///
    /// Each element [pushed](AppendVec::push) to the provided vector will be logically
    /// [inserted](ElementSplice::insert) into `self`.
    fn with_scratch<R>(&mut self, f: impl FnOnce(&mut AppendVec<Element>) -> R) -> R;
    /// Insert a new element at the current index in the resulting collection.
    fn insert(&mut self, element: Element);
    /// Mutate the next existing element.
    fn mutate<R>(&mut self, f: impl FnOnce(Element::Mut<'_>) -> R) -> R;
    /// Don't make any changes to the next n existing elements.
    fn skip(&mut self, n: usize);
    /// How many elements you would need to [`skip`](ElementSplice::skip) from when this
    /// `ElementSplice` was created to get to the current element.
    ///
    /// Note that in using this function, previous views will have skipped.
    /// Values obtained from this method may change during any `rebuild`, but will not change
    /// between `build`/`rebuild` and the next `message`
    fn index(&self) -> usize;
    /// Delete the next existing element, after running a function on it.
    fn delete<R>(&mut self, f: impl FnOnce(Element::Mut<'_>) -> R) -> R;
}

/// An append only `Vec`.
///
/// This will be passed to [`ViewSequence::seq_build`] to
/// build the list of initial elements whilst materializing the sequence.
#[derive(Debug)]
pub struct AppendVec<T> {
    inner: Vec<T>,
}

impl<T> AppendVec<T> {
    /// Convert `self` into the underlying `Vec`
    #[must_use]
    pub fn into_inner(self) -> Vec<T> {
        self.inner
    }
    /// Add an item to the end of the vector.
    pub fn push(&mut self, item: T) {
        self.inner.push(item);
    }
    /// [Drain](Vec::drain) all items from this `AppendVec`.
    pub fn drain(&mut self) -> Drain<'_, T> {
        self.inner.drain(..)
    }
    /// Equivalent to [`ElementSplice::index`].
    pub fn index(&self) -> usize {
        // If there are no items, to get here we need to skip 0
        // if there is one, we need to skip 1
        self.inner.len()
    }
    /// Returns `true` if the vector contains no elements.
    ///
    /// See [`Vec::is_empty`] for more details
    pub fn is_empty(&self) -> bool {
        self.inner.is_empty()
    }
}

impl<T> From<Vec<T>> for AppendVec<T> {
    fn from(inner: Vec<T>) -> Self {
        Self { inner }
    }
}

impl<T> Default for AppendVec<T> {
    fn default() -> Self {
        Self {
            inner: Vec::default(),
        }
    }
}
