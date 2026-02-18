// Copyright 2026 the Xilem Authors
// SPDX-License-Identifier: Apache-2.0

use crate::core::Property;
use crate::util::AnyMap;

/// Mutable reference to a collection of [properties](Property) that a widget has access to.
///
/// Used by the [`Widget`] trait during most passes.
pub struct PropertiesMut<'a> {
    pub(crate) map: &'a mut AnyMap,
    pub(crate) default_map: &'a AnyMap,
}

// TODO - Better document local vs default properties.

impl PropertiesMut<'_> {
    /// Returns `true` if the widget has a local property of type `P`.
    ///
    /// Does not check default properties.
    pub fn contains<P: Property>(&self) -> bool {
        self.map.contains::<P>()
    }

    /// Returns value of property `P`.
    ///
    /// If the widget has an entry for `P`, returns its value.
    /// If the default property map has an entry for `P`, returns its value.
    /// Otherwise returns [`Property::static_default()`].
    pub fn get<P: Property>(&self) -> &P {
        if let Some(p) = self.map.get::<P>() {
            p
        } else if let Some(p) = self.default_map.get::<P>() {
            p
        } else {
            P::static_default()
        }
    }

    /// Returns the defined value of property `P`.
    ///
    /// If the widget has an explicit entry, or the default property map has an explicit entry,
    /// then this will return a value. Otherwise it will return `None`.
    pub fn get_defined<P: Property>(&self) -> Option<&P> {
        self.map.get::<P>().or_else(|| self.default_map.get::<P>())
    }

    /// Sets local property `P` to given value. Returns the previous value if `P` was already set.
    ///
    /// Does not affect default properties.
    ///
    /// If you're using a `WidgetMut`, call [`WidgetMut::insert_prop`] instead.
    ///
    /// [`WidgetMut::insert_prop`]: crate::core::WidgetMut::insert_prop
    pub fn insert<P: Property>(&mut self, value: P) -> Option<P> {
        self.map.insert(value)
    }

    /// Removes local property `P`. Returns the previous value if `P` was set.
    ///
    /// Does not affect default properties.
    ///
    /// If you're using a `WidgetMut`, call [`WidgetMut::remove_prop`] instead.
    ///
    /// [`WidgetMut::remove_prop`]: crate::core::WidgetMut::remove_prop
    pub fn remove<P: Property>(&mut self) -> Option<P> {
        self.map.remove::<P>()
    }

    /// Returns a `PropertiesMut` for the same underlying properties with a shorter lifetime.
    pub fn reborrow_mut(&mut self) -> PropertiesMut<'_> {
        PropertiesMut {
            map: &mut *self.map,
            default_map: self.default_map,
        }
    }
}
