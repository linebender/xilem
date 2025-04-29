// Copyright 2025 the Xilem Authors
// SPDX-License-Identifier: Apache-2.0

use anymap3::{AnyMap, Entry};

/// A marker trait that indicates that a type is intended to be used as a widget's property.
///
/// See [properties documentation](crate::doc::doc_04b_widget_properties) for
/// a full explanation of the general concept.
///
/// Note that if a type `Foobar` implements Property, that tells you that Foobar is meant
/// to be a property of *some* widget, but it doesn't tell you *which* widget accepts Foobar
/// as a property.
/// That information is deliberately not encoded in the type system.
/// We might change that in a future version.
pub trait Property: 'static {}

/// A collection of properties that a widget can be created with.
///
/// See [properties documentation](crate::doc::doc_04b_widget_properties) for details.
#[derive(Default)]
pub struct Properties {
    pub(crate) map: AnyMap,
}

/// Reference to a collection of properties that a widget has access to.
///
/// Used by the [`Widget`] trait during rendering passes and in some search methods.
///
/// See [properties documentation](crate::doc::doc_04b_widget_properties) for
/// details.
///
/// [`Widget`]: crate::core::Widget
#[derive(Clone, Copy)]
pub struct PropertiesRef<'a> {
    pub(crate) map: &'a AnyMap,
}

/// Mutable reference to a collection of properties that a widget has access to.
///
/// Used by the [`Widget`] trait during most passes.
///
/// See [properties documentation](crate::doc::doc_04b_widget_properties) for
/// details.
///
/// [`Widget`]: crate::core::Widget
pub struct PropertiesMut<'a> {
    pub(crate) map: &'a mut AnyMap,
}

impl Properties {
    /// Create an empty collection of properties.
    pub fn new() -> Self {
        Self { map: AnyMap::new() }
    }

    /// Get a reference to the properties.
    pub fn ref_(&self) -> PropertiesRef<'_> {
        PropertiesRef { map: &self.map }
    }

    /// Get a mutable reference to the properties.
    pub fn mut_(&mut self) -> PropertiesMut<'_> {
        PropertiesMut { map: &mut self.map }
    }
}

// TODO - Implement some kind of cascading with at least a Masonry-wide theme,
// If a property is not in the widget *or* the type, return `Default::default()`.
// Don't return Option types anymore.

impl PropertiesRef<'_> {
    /// Returns `true` if the widget has a property of type `T`.
    pub fn contains<T: Property>(&self) -> bool {
        self.map.contains::<T>()
    }

    /// Get value of property `T`, or `None` if the widget has no `T` property.
    pub fn get<T: Property>(&self) -> Option<&T> {
        self.map.get::<T>()
    }
}

impl PropertiesMut<'_> {
    /// Returns `true` if the widget has a property of type `T`.
    pub fn contains<T: Property>(&self) -> bool {
        self.map.contains::<T>()
    }

    /// Get value of property `T`, or `None` if the widget has no `T` property.
    pub fn get<T: Property>(&self) -> Option<&T> {
        self.map.get::<T>()
    }

    /// Get value of property `T`, or `None` if the widget has no `T` property.
    ///
    /// If you're using a `WidgetMut`, call [`WidgetMut::get_prop_mut`] instead.
    ///
    /// [`WidgetMut::get_prop_mut`]: crate::core::WidgetMut::get_prop_mut
    pub fn get_mut<T: Property>(&mut self) -> Option<&mut T> {
        self.map.get_mut::<T>()
    }

    /// Set property `T` to given value. Returns the previous value if `T` was already set.
    ///
    /// If you're using a `WidgetMut`, call [`WidgetMut::insert_prop`] instead.
    ///
    /// [`WidgetMut::insert_prop`]: crate::core::WidgetMut::insert_prop
    pub fn insert<T: Property>(&mut self, value: T) -> Option<T> {
        self.map.insert(value)
    }

    /// Remove property `T`. Returns the previous value if `T` was set.
    ///
    /// If you're using a `WidgetMut`, call [`WidgetMut::remove_prop`] instead.
    ///
    /// [`WidgetMut::remove_prop`]: crate::core::WidgetMut::remove_prop
    pub fn remove<T: Property>(&mut self) -> Option<T> {
        self.map.remove::<T>()
    }

    /// Returns an entry that can be used to add, update, or remove a property.
    ///
    /// If you're using a `WidgetMut`, call [`WidgetMut::prop_entry`] instead.
    ///
    /// [`WidgetMut::prop_entry`]: crate::core::WidgetMut::prop_entry
    pub fn entry<T: Property>(&mut self) -> Entry<'_, dyn std::any::Any, T> {
        self.map.entry::<T>()
    }

    /// Get a `PropertiesMut` for the same underlying properties with a shorter lifetime.
    pub fn reborrow_mut(&mut self) -> PropertiesMut<'_> {
        PropertiesMut {
            map: &mut *self.map,
        }
    }
}
