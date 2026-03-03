// Copyright 2026 the Xilem Authors
// SPDX-License-Identifier: Apache-2.0

use crate::core::{ClassSet, Property, PropertySelection, PropertySet, PropertyStack};
use crate::util::AnyMap;

/// Mutable reference to a collection of [properties](Property) that a widget has access to.
///
/// Used by the [`Widget`](crate::core::Widget) trait during most passes.
pub struct PropertiesMut<'a> {
    pub(crate) set: &'a mut PropertySet,
    pub(crate) default_map: &'a AnyMap,
    pub(crate) stack: Option<&'a PropertyStack>,
    pub(crate) class_set: &'a ClassSet,
    pub(crate) selection: &'a mut PropertySelection,
}

// TODO - Better document local vs default properties.

impl PropertiesMut<'_> {
    /// Returns `true` if the widget has a local property of type `P`.
    ///
    /// Does not check default properties.
    pub fn contains<P: Property>(&self) -> bool {
        self.set.map.contains::<P>()
    }

    /// Returns value of property `P`.
    ///
    /// Checks local properties first, then the property stack (cache write-through),
    /// then default properties, then [`Property::static_default()`].
    pub fn get<P: Property>(&mut self) -> &P {
        // 1. Local properties
        if let Some(p) = self.set.map.get::<P>() {
            return p;
        }
        // 2. Property stack (writes to cache and relevance tracking on miss)
        if let Some(stack) = self.stack {
            if let Some(p) = stack.resolve_cached_mut::<P>(self.selection, self.class_set) {
                return p;
            }
        }
        // 3. Default properties
        if let Some(p) = self.default_map.get::<P>() {
            return p;
        }
        // 4. Static default
        P::static_default()
    }

    /// Returns the defined value of property `P`.
    ///
    /// If the widget has an explicit entry, or the default property map has an explicit entry,
    /// then this will return a value. Otherwise it will return `None`.
    pub fn get_defined<P: Property>(&self) -> Option<&P> {
        self.set
            .map
            .get::<P>()
            .or_else(|| self.default_map.get::<P>())
    }

    /// Sets local property `P` to given value. Returns the previous value if `P` was already set.
    ///
    /// Does not affect default properties.
    ///
    /// If you're using a `WidgetMut`, call [`WidgetMut::insert_prop`] instead.
    ///
    /// [`WidgetMut::insert_prop`]: crate::core::WidgetMut::insert_prop
    pub fn insert<P: Property>(&mut self, value: P) -> Option<P> {
        self.set.map.insert(value)
    }

    /// Removes local property `P`. Returns the previous value if `P` was set.
    ///
    /// Does not affect default properties.
    ///
    /// If you're using a `WidgetMut`, call [`WidgetMut::remove_prop`] instead.
    ///
    /// [`WidgetMut::remove_prop`]: crate::core::WidgetMut::remove_prop
    pub fn remove<P: Property>(&mut self) -> Option<P> {
        self.set.map.remove::<P>()
    }

    /// Returns a mutable reference to the local properties for direct access.
    pub fn local_properties(&mut self) -> &mut PropertySet {
        self.set
    }

    /// Returns a `PropertiesMut` for the same underlying properties with a shorter lifetime.
    pub fn reborrow_mut(&mut self) -> PropertiesMut<'_> {
        PropertiesMut {
            set: &mut *self.set,
            default_map: self.default_map,
            stack: self.stack,
            class_set: self.class_set,
            selection: &mut *self.selection,
        }
    }
}
