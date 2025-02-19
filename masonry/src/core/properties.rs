// Copyright 2025 the Xilem Authors
// SPDX-License-Identifier: Apache-2.0

#![allow(missing_docs, reason = "TODO")]

use anymap3::{AnyMap, Entry};

use crate::core::MutateCtx;

pub trait WidgetProperty: 'static {
    fn changed(ctx: &mut MutateCtx<'_>);
}

// TODO - Add PropertyValue wrapper struct that implements receiver trait.
// Return PropertyValue<T> instead of Option<T> from methods.

#[derive(Default)]
pub struct Properties {
    pub(crate) map: AnyMap,
}

#[derive(Clone, Copy)]
pub struct PropertiesRef<'a> {
    pub(crate) map: &'a AnyMap,
}

pub struct PropertiesMut<'a> {
    pub(crate) map: &'a mut AnyMap,
}

impl Properties {
    pub fn new() -> Self {
        Self { map: AnyMap::new() }
    }

    pub fn ref_(&self) -> PropertiesRef<'_> {
        PropertiesRef { map: &self.map }
    }

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
    /// [WidgetMut::get_mut]: crate::core::WidgetMut::get_prop_mut
    pub fn get_mut<T: WidgetProperty>(&mut self) -> Option<&mut T> {
        self.map.get_mut::<T>()
    }

    /// Set property `T` to given value. Returns the previous value if `T` was already set.
    ///
    /// If you're using a `WidgetMut`, call [`WidgetMut::insert_prop`] instead.
    ///
    /// [WidgetMut::insert]: crate::core::WidgetMut::insert_prop
    pub fn insert<T: WidgetProperty>(&mut self, value: T) -> Option<T> {
        self.map.insert(value)
    }

    /// Remove property `T`. Returns the previous value if `T` was set.
    ///
    /// If you're using a `WidgetMut`, call [`WidgetMut::remove_prop`] instead.
    ///
    /// [WidgetMut::remove]: crate::core::WidgetMut::remove_prop
    pub fn remove<T: WidgetProperty>(&mut self) -> Option<T> {
        self.map.remove::<T>()
    }

    /// Returns an entry that can be used to add, update, or remove a property.
    ///
    /// If you're using a `WidgetMut`, call [`WidgetMut::prop_entry`] instead.
    ///
    /// [WidgetMut::entry]: crate::core::WidgetMut::prop_entry
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
