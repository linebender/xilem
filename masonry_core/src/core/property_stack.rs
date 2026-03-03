// Copyright 2026 the Xilem Authors
// SPDX-License-Identifier: Apache-2.0

use std::any::TypeId;
use std::fmt::Display;
use std::num::NonZeroU64;
use std::sync::atomic::{AtomicU64, Ordering};

use crate::core::{ClassSet, Property, PropertySelection, PropertySet, Selector};

/// A unique identifier for a single [`PropertyStack`].
#[derive(Clone, Copy, Debug, Hash, PartialEq, Eq)]
pub struct PropertyStackId(pub(crate) NonZeroU64);

/// A cascading set of properties that can be applied to widgets.
///
/// Each layer of the stack consists of a [`Selector`] and a set of properties.
/// When resolving a property, the stack is traversed from top to bottom until
/// a matching selector with the requested property is found.
#[derive(Default)]
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
    pub fn new() -> Self {
        Self::default()
    }

    /// Pushes a new entry onto the stack.
    ///
    /// The selector is used to determine whether the entry applies to a given widget based on its class set.
    pub fn push(&mut self, selector: Selector, properties: PropertySet) {
        self.stack.push((selector, properties));
    }

    pub(crate) fn resolve<P: Property>(&self, classes: &ClassSet) -> Option<usize> {
        // Iter over items and indices
        for (i, (selector, prop_set)) in self.stack.iter().enumerate().rev() {
            if selector.matches(classes) && prop_set.map.contains::<P>() {
                return Some(i);
            }
        }
        None
    }

    pub(crate) fn resolve_cached<P: Property>(
        &self,
        selected: &PropertySelection,
        classes: &ClassSet,
    ) -> Option<&P> {
        let index = selected.selected.get(&TypeId::of::<P>()).copied();
        let index = index.or_else(|| self.resolve::<P>(classes))?;

        let Some(item) = self.stack[index].1.get::<P>() else {
            debug_panic!("Invalid PropertySelection cache");
            return None;
        };
        Some(item)
    }

    // TODO - Refactor with resolve_cached? Overall this is ugly code.
    pub(crate) fn resolve_cached_mut<P: Property>(
        &self,
        selected: &mut PropertySelection,
        classes: &ClassSet,
    ) -> Option<&P> {
        let mut index = selected.selected.get(&TypeId::of::<P>()).copied();
        if index.is_none() {
            index = self.resolve::<P>(classes);
            if let Some(i) = index {
                selected.selected.insert(TypeId::of::<P>(), i);
                selected.extend_relevant(&self.stack[i].0);
            }
        }
        let index = index?;

        let Some(item) = self.stack[index].1.get::<P>() else {
            debug_panic!("Invalid PropertySelection cache");
            return None;
        };
        Some(item)
    }
}
