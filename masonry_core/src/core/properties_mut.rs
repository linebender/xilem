// Copyright 2026 the Xilem Authors
// SPDX-License-Identifier: Apache-2.0

use crate::core::{ClassSet, Property, PropertyCache, PropertySet, PropertyStack};
use crate::util::AnyMap;

/// Mutable reference to a collection of [properties](Property) that a widget has access to.
///
/// Used by the [`Widget`](crate::core::Widget) trait during most passes.
pub struct PropertiesMut<'a> {
    pub(crate) local: &'a mut PropertySet,
    pub(crate) default_map: &'a AnyMap,
    pub(crate) stack: &'a PropertyStack,
    pub(crate) class_set: &'a ClassSet,
}

// TODO - Better document local vs default properties.

impl PropertiesMut<'_> {
    /// Returns `true` if the widget has a local property of type `P`.
    ///
    /// Does not check default properties.
    pub fn contains<P: Property>(&self) -> bool {
        self.local.map.contains::<P>()
    }

    /// Returns value of property `P`.
    ///
    /// Checks local properties first, then the property stack (cache write-through),
    /// then default properties, then [`Property::static_default()`].
    ///
    /// The returned `&P` borrows from `self`, not from `cache`, so multiple
    /// `get` results can be held simultaneously as long as each call re-borrows
    /// the cache independently.
    pub fn get<P: Property>(&self, cache: &mut PropertyCache) -> &P {
        // 1. Local properties
        if let Some(p) = self.local.map.get::<P>() {
            return p;
        }
        // 2. Property stack (writes to cache and relevance tracking on miss)
        if let Some(p) = self.stack.resolve_cached_mut::<P>(cache, self.class_set) {
            return p;
        }
        // 3. Default properties
        if let Some(p) = self.default_map.get::<P>() {
            return p;
        }
        // 4. Static default
        P::static_default()
    }

    /// Warms the [`PropertyStack`] cache for property `P`.
    ///
    /// Searches the stack for a matching entry and, if found, records it in
    /// the cache.
    /// Subsequent calls to [`get_cached`](Self::get_cached) will use the
    /// cached result without requiring a mutable borrow on the cache.
    pub fn resolve<P: Property>(&self, cache: &mut PropertyCache) {
        let _ = self.stack.resolve_cached_mut::<P>(cache, self.class_set);
    }

    /// Returns a shared reference to property `P` using the cached stack resolution.
    ///
    /// Because this takes `&PropertyCache` (shared), multiple results from
    /// `get_cached` may be held simultaneously.
    ///
    /// **Prerequisite:** call [`resolve`](Self::resolve) first for any property
    /// that may come from the [`PropertyStack`]. Without a prior `resolve`, the
    /// cache may not be populated and relevance tracking will not be updated,
    /// which can cause `run_update_props_pass` to miss cache invalidations.
    ///
    /// For a single property lookup that does not need to be held alongside
    /// another, prefer [`get`](Self::get) directly.
    pub fn get_cached<P: Property>(&self, cache: &PropertyCache) -> &P {
        if !cache.is_cached::<P>() {
            debug_panic!(
                "Property {} was not resolved before get_cached",
                std::any::type_name::<P>()
            );
        }
        // 1. Local properties (always accessible; no stack involvement)
        if let Some(p) = self.local.map.get::<P>() {
            return p;
        }
        // 2. Property stack (reads from cache; linear scan as fallback
        //    if resolve was not called, but does not update the cache or relevance)
        if let Some(p) = self.stack.resolve_cached::<P>(cache, self.class_set) {
            return p;
        }
        // 3. Default properties
        if let Some(p) = self.default_map.get::<P>() {
            return p;
        }
        // 4. Static default
        P::static_default()
    }

    /// Sets local property `P` to given value. Returns the previous value if `P` was already set.
    ///
    /// Does not affect default properties.
    ///
    /// If you're using a `WidgetMut`, call [`WidgetMut::insert_prop`] instead.
    ///
    /// [`WidgetMut::insert_prop`]: crate::core::WidgetMut::insert_prop
    pub fn insert<P: Property>(&mut self, value: P) -> Option<P> {
        self.local.map.insert(value)
    }

    /// Removes local property `P`. Returns the previous value if `P` was set.
    ///
    /// Does not affect default properties.
    ///
    /// If you're using a `WidgetMut`, call [`WidgetMut::remove_prop`] instead.
    ///
    /// [`WidgetMut::remove_prop`]: crate::core::WidgetMut::remove_prop
    pub fn remove<P: Property>(&mut self) -> Option<P> {
        self.local.map.remove::<P>()
    }

    /// Returns a reference to the local properties for direct access.
    pub fn local_properties(&mut self) -> &mut PropertySet {
        self.local
    }
}
