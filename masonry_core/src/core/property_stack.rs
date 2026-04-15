// Copyright 2026 the Xilem Authors
// SPDX-License-Identifier: Apache-2.0

use std::any::TypeId;
use std::fmt::Display;
use std::num::NonZeroU64;
use std::sync::atomic::{AtomicU64, Ordering};

use crate::core::{ClassSet, Property, PropertyCache, PropertySet, Selector};

/// A unique identifier for a single [`PropertyStack`].
#[derive(Clone, Copy, Debug, Hash, PartialEq, Eq)]
pub struct PropertyStackId(pub(crate) NonZeroU64);

/// A cascading set of properties that can be applied to widgets.
///
/// Each layer of the stack consists of a [`Selector`] and a set of properties.
/// When resolving a property, the stack is traversed from top to bottom until
/// a matching selector with the requested property is found.
#[derive(Debug, Default)]
pub struct PropertyStack {
    pub(crate) stack: Vec<(Selector, PropertySet)>,
}

// ---

impl PropertyStackId {
    /// Allocates a new, unique `PropertyStackId`.
    pub fn next() -> Self {
        static PROPERTY_STACK_ID_COUNTER: AtomicU64 = AtomicU64::new(1);
        let id = PROPERTY_STACK_ID_COUNTER.fetch_add(1, Ordering::Relaxed);
        Self(id.try_into().unwrap())
    }

    /// Returns the integer value of the `PropertyStackId`.
    pub fn to_raw(self) -> u64 {
        self.0.into()
    }
}

impl From<PropertyStackId> for u64 {
    fn from(id: PropertyStackId) -> Self {
        id.0.into()
    }
}

impl Display for PropertyStackId {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        write!(f, "#{}", self.0)
    }
}

// ---

impl PropertyStack {
    /// Creates an empty `PropertyStack`.
    pub const fn new() -> Self {
        Self { stack: Vec::new() }
    }

    /// Pushes a new entry onto the stack.
    ///
    /// The selector is used to determine whether the entry applies to a given widget based on its class set.
    pub fn push(&mut self, selector: Selector, properties: impl Into<PropertySet>) {
        self.stack.push((selector, properties.into()));
    }

    fn get_prop<P: Property>(&self, maybe_index: Option<usize>) -> Option<&P> {
        let Some(index) = maybe_index else {
            // We've cached/resolved that there is no matching entry in the stack.
            return None;
        };
        let Some(item) = self.stack[index].1.get::<P>() else {
            debug_panic!("Invalid PropertyStack index - probably a bug in PropertyCache logic");
            return None;
        };
        Some(item)
    }

    pub(crate) fn resolve_index(&self, classes: &ClassSet, prop_type: TypeId) -> Option<usize> {
        // Iterate from top to bottom to enable property shadowing.
        for (i, (selector, prop_set)) in self.stack.iter().enumerate().rev() {
            if selector.matches(classes) && prop_set.map.as_raw().contains_key(&prop_type) {
                return Some(i);
            }
        }
        None
    }

    pub(crate) fn resolve<P: Property>(
        &self,
        cache: &mut PropertyCache,
        classes: &ClassSet,
    ) -> Option<&P> {
        // If cached, return cached result.
        if let Some(cached_index) = cache.cached_index(TypeId::of::<P>()) {
            return self.get_prop::<P>(cached_index);
        }

        // Else, update cache and return result.
        for (i, (selector, prop_set)) in self.stack.iter().enumerate().rev() {
            cache.extend_relevant(selector);

            if selector.matches(classes)
                && let Some(item) = prop_set.map.get::<P>()
            {
                cache.entries.insert(TypeId::of::<P>(), Some(i));
                return Some(item);
            }
        }

        cache.entries.insert(TypeId::of::<P>(), None);
        None
    }

    pub(crate) fn resolve_without_saving<P: Property>(
        &self,
        cache: &PropertyCache,
        classes: &ClassSet,
    ) -> Option<&P> {
        // If cached, return cached result.
        if let Some(cached_index) = cache.cached_index(TypeId::of::<P>()) {
            return self.get_prop::<P>(cached_index);
        }

        // Else, return result without updating cache.
        let index = self.resolve_index(classes, TypeId::of::<P>());
        self.get_prop::<P>(index)
    }
}
