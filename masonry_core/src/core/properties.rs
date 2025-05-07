// Copyright 2025 the Xilem Authors
// SPDX-License-Identifier: Apache-2.0

use crate::util::AnySendMap;

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
pub trait Property: Send + Sync + 'static {}

/// A collection of properties that a widget can be created with.
///
/// See [properties documentation](crate::doc::doc_04b_widget_properties) for details.
#[derive(Default)]
pub struct Properties {
    pub(crate) map: AnySendMap,
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
    pub(crate) map: &'a AnySendMap,
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
    pub(crate) map: &'a mut AnySendMap,
}

impl Properties {
    /// Create an empty collection of properties.
    pub fn new() -> Self {
        Self {
            map: AnySendMap::new(),
        }
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

// TODO - If a property is not in the widget *or* the type, return `Default::default()`.
// Don't return Option types anymore.

impl PropertiesRef<'_> {
    /// Returns `true` if the widget has a property of type `P`.
    pub fn contains<P: Property>(&self) -> bool {
        self.map.contains::<P>()
    }

    /// Get value of property `P`, or `None` if the widget has no `P` property.
    pub fn get<P: Property>(&self) -> Option<&P> {
        self.map.get::<P>()
    }
}

impl PropertiesMut<'_> {
    /// Returns `true` if the widget has a property of type `P`.
    pub fn contains<P: Property>(&self) -> bool {
        self.map.contains::<P>()
    }

    /// Get value of property `P`, or `None` if the widget has no `P` property.
    pub fn get<P: Property>(&self) -> Option<&P> {
        self.map.get::<P>()
    }

    /// Get value of property `P`, or `None` if the widget has no `P` property.
    ///
    /// If you're using a `WidgetMut`, call [`WidgetMut::get_prop_mut`] instead.
    ///
    /// [`WidgetMut::get_prop_mut`]: crate::core::WidgetMut::get_prop_mut
    pub fn get_mut<P: Property>(&mut self) -> Option<&mut P> {
        self.map.get_mut::<P>()
    }

    /// Set property `P` to given value. Returns the previous value if `P` was already set.
    ///
    /// If you're using a `WidgetMut`, call [`WidgetMut::insert_prop`] instead.
    ///
    /// [`WidgetMut::insert_prop`]: crate::core::WidgetMut::insert_prop
    pub fn insert<P: Property>(&mut self, value: P) -> Option<P> {
        self.map.insert(value)
    }

    /// Remove property `P`. Returns the previous value if `P` was set.
    ///
    /// If you're using a `WidgetMut`, call [`WidgetMut::remove_prop`] instead.
    ///
    /// [`WidgetMut::remove_prop`]: crate::core::WidgetMut::remove_prop
    pub fn remove<P: Property>(&mut self) -> Option<P> {
        self.map.remove::<P>()
    }

    /// Get a `PropertiesMut` for the same underlying properties with a shorter lifetime.
    pub fn reborrow_mut(&mut self) -> PropertiesMut<'_> {
        PropertiesMut {
            map: &mut *self.map,
        }
    }
}
