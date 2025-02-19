// Copyright 2025 the Xilem Authors
// SPDX-License-Identifier: Apache-2.0

use anymap3::{AnyMap, Entry};

use crate::core::MutateCtx;

/// A property that can be set on a widget.
///
/// See [properties documentation](crate::doc::doc_03_implementing_container_widget) for details.
pub trait WidgetProperty: 'static {
    /// Called when the property is inserted, mutated or removed.
    ///
    /// Should set invalidation flags (relayout, repaint, etc) on the given context.
    fn changed(ctx: &mut MutateCtx<'_>);
}

// TODO - Add PropertyValue wrapper struct that implements receiver trait.
// Return PropertyValue<T> instead of Option<T> from methods.

/// A collection of properties that a widget can be created with.
///
/// See [properties documentation](crate::doc::doc_03_implementing_container_widget) for details.
#[derive(Default)]
pub struct Properties {
    pub(crate) map: AnyMap,
}

/// Reference to a collection of properties that a widget has access to.
///
/// Used by the [`Widget`] trait during rendering passes and in some search methods.
///
/// See [properties documentation](crate::doc::doc_03_implementing_container_widget) for
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
/// See [properties documentation](crate::doc::doc_03_implementing_container_widget) for
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

impl PropertiesRef<'_> {
    /// Returns true if the widget has a property of type `T`.
    pub fn contains<T: WidgetProperty>(&self) -> bool {
        self.map.contains::<T>()
    }

    /// Get value of property `T`, or None if the widget has no `T` property.
    pub fn get<T: WidgetProperty>(&self) -> Option<&T> {
        self.map.get::<T>()
    }
}

impl PropertiesMut<'_> {
    /// Returns true if the widget has a property of type `T`.
    pub fn contains<T: WidgetProperty>(&self) -> bool {
        self.map.contains::<T>()
    }

    /// Get value of property `T`, or None if the widget has no `T` property.
    pub fn get<T: WidgetProperty>(&self) -> Option<&T> {
        self.map.get::<T>()
    }

    /// Get value of property `T`, or None if the widget has no `T` property.
    ///
    /// If you're using a `WidgetMut`, call [`WidgetMut::get_prop_mut`] instead.
    ///
    /// [`WidgetMut::get_prop_mut`]: crate::core::WidgetMut::get_prop_mut
    pub fn get_mut<T: WidgetProperty>(&mut self) -> Option<&mut T> {
        self.map.get_mut::<T>()
    }

    /// Set property `T` to given value. Returns the previous value if `T` was already set.
    ///
    /// If you're using a `WidgetMut`, call [`WidgetMut::insert_prop`] instead.
    ///
    /// [`WidgetMut::insert_prop`]: crate::core::WidgetMut::insert_prop
    pub fn insert<T: WidgetProperty>(&mut self, value: T) -> Option<T> {
        self.map.insert(value)
    }

    /// Remove property `T`. Returns the previous value if `T` was set.
    ///
    /// If you're using a `WidgetMut`, call [`WidgetMut::remove_prop`] instead.
    ///
    /// [`WidgetMut::remove_prop`]: crate::core::WidgetMut::remove_prop
    pub fn remove<T: WidgetProperty>(&mut self) -> Option<T> {
        self.map.remove::<T>()
    }

    /// Returns an entry that can be used to add, update, or remove a property.
    ///
    /// If you're using a `WidgetMut`, call [`WidgetMut::prop_entry`] instead.
    ///
    /// [`WidgetMut::prop_entry`]: crate::core::WidgetMut::prop_entry
    pub fn entry<T: WidgetProperty>(&mut self) -> Entry<'_, dyn std::any::Any, T> {
        self.map.entry::<T>()
    }

    /// Get a `PropertiesMut` for the same underlying properties with a shorter lifetime.
    pub fn reborrow_mut(&mut self) -> PropertiesMut<'_> {
        PropertiesMut {
            map: &mut *self.map,
        }
    }
}
