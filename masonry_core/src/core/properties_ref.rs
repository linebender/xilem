// Copyright 2026 the Xilem Authors
// SPDX-License-Identifier: Apache-2.0

use crate::core::{ClassSet, Property, PropertyCache, PropertySet, PropertyStack};
use crate::util::AnyMap;

/// Reference to a collection of [properties](Property) that a widget has access to.
///
/// Used by the [`Widget`](crate::core::Widget) trait during rendering passes and in some search methods.
#[derive(Clone, Copy)]
pub struct PropertiesRef<'a> {
    pub(crate) local: &'a PropertySet,
    pub(crate) default_map: &'a AnyMap,
    pub(crate) stack: &'a PropertyStack,
    pub(crate) class_set: &'a ClassSet,
}

// TODO - Better document local vs default properties.

impl PropertiesRef<'_> {
    /// Returns `true` if the widget has a local property of type `P`.
    ///
    /// Does not check default properties.
    pub fn contains<P: Property>(&self) -> bool {
        self.local.map.contains::<P>()
    }

    /// Returns value of property `P`.
    ///
    /// Checks local properties first, then the property stack (cache read only),
    /// then default properties, then [`Property::static_default()`].
    pub fn get<P: Property>(&self, cache: &mut PropertyCache) -> &P {
        // 1. Local properties
        if let Some(p) = self.local.map.get::<P>() {
            return p;
        }
        // 2. Property stack (cache read only; linear scan on cache miss)
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

    pub(crate) fn get_without_saving<P: Property>(&self, cache: &PropertyCache) -> &P {
        // 1. Local properties
        if let Some(p) = self.local.map.get::<P>() {
            return p;
        }
        // 2. Property stack (cache read only; linear scan on cache miss)
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

    /// Returns a reference to the local properties for direct access.
    pub fn local_properties(&self) -> &PropertySet {
        self.local
    }
}
