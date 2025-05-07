// Copyright 2025 the Xilem Authors
// SPDX-License-Identifier: Apache-2.0

use std::any::{Any, TypeId};
use std::collections::HashMap;
use std::default::Default;

use crate::core::Widget;
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
pub trait Property: Default + Send + Sync + 'static {
    /// A default value that can be stored in statics.
    ///
    /// Should be the same as [`Default::default()`].
    ///
    /// **Note:** This is a hacky workaround until we find a better way to store default
    /// values for properties.
    const DEFAULT: Self;
}

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
    pub(crate) default_map: &'a AnySendMap,
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
    pub(crate) default_map: &'a AnySendMap,
}

// TODO - Document default properties.
/// A collection of default properties for all widgets.
///
/// Default property values can be added to this collection for
/// every `(widget type, property type)` pair.
///
/// See [properties documentation](crate::doc::doc_04b_widget_properties) for details.
#[derive(Default, Debug)]
pub struct DefaultProperties {
    /// Maps widget types to the default property set for that widget.
    pub(crate) map: HashMap<TypeId, AnySendMap>,
    pub(crate) dummy_map: AnySendMap,
}

impl Properties {
    /// Create an empty collection of properties.
    pub fn new() -> Self {
        Self {
            map: AnySendMap::new(),
        }
    }

    /// Returns `true` if the set has a property of type `P`.
    pub fn contains<P: Property>(&self) -> bool {
        self.map.contains::<P>()
    }

    /// Get value of property `P`.
    pub fn get<P: Property>(&self) -> Option<&P> {
        self.map.get::<P>()
    }

    /// Set property `P` to given value. Returns the previous value if `P` was already set.
    pub fn insert<P: Property>(&mut self, value: P) -> Option<P> {
        self.map.insert(value)
    }

    /// Remove property `P`. Returns the previous value if `P` was set.
    pub fn remove<P: Property>(&mut self) -> Option<P> {
        self.map.remove::<P>()
    }
}

// TODO - If a property is not in the widget *or* the type, return `Default::default()`.
// Don't return Option types anymore.

impl PropertiesRef<'_> {
    /// Returns `true` if the widget has a property of type `P`.
    ///
    /// Does not check default properties.
    pub fn contains<P: Property>(&self) -> bool {
        self.map.contains::<P>()
    }

    /// Get value of property `P`.
    ///
    /// Returns Some if either the widget or the default property set has an entry for `P`.
    /// Returns `None` otherwise.
    pub fn get<P: Property>(&self) -> Option<&P> {
        self.map.get::<P>().or_else(|| self.default_map.get::<P>())
    }
}

impl PropertiesMut<'_> {
    /// Returns `true` if the widget has a property of type `P`.
    ///
    /// Does not check default properties.
    pub fn contains<P: Property>(&self) -> bool {
        self.map.contains::<P>()
    }

    /// Get value of property `P`.
    ///
    /// Returns Some if either the widget or the default property set has an entry for `P`.
    /// Returns `None` otherwise.
    pub fn get<P: Property>(&self) -> Option<&P> {
        self.map.get::<P>().or_else(|| self.default_map.get::<P>())
    }

    /// Set property `P` to given value. Returns the previous value if `P` was already set.
    ///
    /// Does not affect default properties.
    ///
    /// If you're using a `WidgetMut`, call [`WidgetMut::insert_prop`] instead.
    ///
    /// [`WidgetMut::insert_prop`]: crate::core::WidgetMut::insert_prop
    pub fn insert<P: Property>(&mut self, value: P) -> Option<P> {
        self.map.insert(value)
    }

    /// Remove property `P`. Returns the previous value if `P` was set.
    ///
    /// Does not affect default properties.
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
            default_map: self.default_map,
        }
    }
}

impl DefaultProperties {
    /// Create an empty set with no default values.
    ///
    /// A completely empty set is probably not what you want.
    /// It means buttons will be displayed without borders or backgrounds, textboxes won't
    /// have default padding, etc.
    /// You should either add a thorough set of values to this, or start from an existing set.
    pub fn new() -> Self {
        Self {
            map: HashMap::new(),
            dummy_map: AnySendMap::new(),
        }
    }

    /// Set the default value of property `P` for widget `W`.
    ///
    /// Widgets for which the property `P` isn't set will get `value` instead.
    pub fn insert<W: Widget, P: Property>(&mut self, value: P) -> Option<P> {
        self.map.entry(TypeId::of::<W>()).or_default().insert(value)
    }

    pub(crate) fn for_widget(&self, id: TypeId) -> &AnySendMap {
        self.map.get(&id).unwrap_or(&self.dummy_map)
    }
}
