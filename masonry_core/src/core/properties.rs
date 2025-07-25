// Copyright 2025 the Xilem Authors
// SPDX-License-Identifier: Apache-2.0

use std::any::TypeId;
use std::collections::HashMap;
use std::default::Default;

use crate::core::Widget;
use crate::util::AnyMap;

/// A marker trait that indicates that a type is intended to be used as a widget's property.
///
/// Properties are arbitrary values that are stored alongside a widget.
///
/// Note that if a type `Foobar` implements `Property`, that tells you that `Foobar` is meant
/// to be a property of *some* widget, but it doesn't tell you *which* widget accepts `Foobar`
/// as a property.
/// That information is deliberately not encoded in the type system.
/// We might change that in a future version.
pub trait Property: Default + Send + Sync + 'static {
    /// A static reference to a default value.
    ///
    /// Should be the same as [`Default::default()`].
    ///
    /// The reason we have this method is that we want e.g. [`PropertiesRef::get()`] to
    /// return a `&T`, not an `Option<&T>`.
    ///
    /// Therefore `Property` must provide a method that will return a `&'static T` that we
    /// can use anywhere.
    /// We do that is by creating a static inside each impl, and returning a reference to that static.
    ///
    /// Ideally, when const generics are stable, we'll want to use `const Default` directly in the default impl.
    fn static_default() -> &'static Self;
}

// TODO - Implement Debug.
/// A collection of [properties](Property) that a widget can be created with.
#[derive(Default)]
pub struct Properties {
    pub(crate) map: AnyMap,
}

/// Reference to a collection of [properties](Property) that a widget has access to.
///
/// Used by the [`Widget`] trait during rendering passes and in some search methods.
#[derive(Clone, Copy)]
pub struct PropertiesRef<'a> {
    pub(crate) map: &'a AnyMap,
    pub(crate) default_map: &'a AnyMap,
}

/// Mutable reference to a collection of [properties](Property) that a widget has access to.
///
/// Used by the [`Widget`] trait during most passes.
pub struct PropertiesMut<'a> {
    pub(crate) map: &'a mut AnyMap,
    pub(crate) default_map: &'a AnyMap,
}

// TODO - Better document local vs default properties.

/// A collection of default [properties](Property) for all widgets.
///
/// Default property values can be added to this collection for
/// every `(widget type, property type)` pair.
#[derive(Default, Debug)]
pub struct DefaultProperties {
    /// Maps widget types to the default property map for that widget.
    pub(crate) map: HashMap<TypeId, AnyMap>,
    pub(crate) dummy_map: AnyMap,
}

impl Properties {
    /// Create an empty collection of properties.
    pub fn new() -> Self {
        Self { map: AnyMap::new() }
    }

    /// Builder-style method to add a property `P` with the given value.
    ///
    /// If the value was already set, it's discarded and replaced with the new value.
    pub fn with<P: Property>(mut self, value: P) -> Self {
        self.map.insert(value);
        self
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

macro_rules! impl_props_from_tuple {
    (
        $(
            $Type: ident, $idx: tt;
        )*
    ) => {

        impl<
            $($Type: Property,)*
        >
        From<( $($Type,)* )> for Properties
        {
            fn from(value: ( $($Type,)* )) -> Self {
                Properties::new()
                    $(
                        .with(value.$idx)
                    )*
            }
        }

    };
}

impl_props_from_tuple!(P0, 0; P1, 1; P2, 2; P3, 3; P4, 4; P5, 5; P6, 6; P7, 7; P8, 8; P9, 9; P10, 10; P11, 11;);
impl_props_from_tuple!(P0, 0; P1, 1; P2, 2; P3, 3; P4, 4; P5, 5; P6, 6; P7, 7; P8, 8; P9, 9; P10, 10;);
impl_props_from_tuple!(P0, 0; P1, 1; P2, 2; P3, 3; P4, 4; P5, 5; P6, 6; P7, 7; P8, 8; P9, 9;);
impl_props_from_tuple!(P0, 0; P1, 1; P2, 2; P3, 3; P4, 4; P5, 5; P6, 6; P7, 7; P8, 8;);
impl_props_from_tuple!(P0, 0; P1, 1; P2, 2; P3, 3; P4, 4; P5, 5; P6, 6; P7, 7;);
impl_props_from_tuple!(P0, 0; P1, 1; P2, 2; P3, 3; P4, 4; P5, 5; P6, 6;);
impl_props_from_tuple!(P0, 0; P1, 1; P2, 2; P3, 3; P4, 4; P5, 5;);
impl_props_from_tuple!(P0, 0; P1, 1; P2, 2; P3, 3; P4, 4;);
impl_props_from_tuple!(P0, 0; P1, 1; P2, 2; P3, 3;);
impl_props_from_tuple!(P0, 0; P1, 1; P2, 2;);
impl_props_from_tuple!(P0, 0; P1, 1;);
impl_props_from_tuple!(P0, 0;);

impl PropertiesRef<'_> {
    /// Returns `true` if the widget has a local property of type `P`.
    ///
    /// Does not check default properties.
    pub fn contains<P: Property>(&self) -> bool {
        self.map.contains::<P>()
    }

    /// Get value of property `P`.
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
}

impl PropertiesMut<'_> {
    /// Returns `true` if the widget has a local property of type `P`.
    ///
    /// Does not check default properties.
    pub fn contains<P: Property>(&self) -> bool {
        self.map.contains::<P>()
    }

    /// Get value of property `P`.
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

    /// Set local property `P` to given value. Returns the previous value if `P` was already set.
    ///
    /// Does not affect default properties.
    ///
    /// If you're using a `WidgetMut`, call [`WidgetMut::insert_prop`] instead.
    ///
    /// [`WidgetMut::insert_prop`]: crate::core::WidgetMut::insert_prop
    pub fn insert<P: Property>(&mut self, value: P) -> Option<P> {
        self.map.insert(value)
    }

    /// Remove local property `P`. Returns the previous value if `P` was set.
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
    /// Create an empty property map with no default values.
    ///
    /// A completely empty property map is probably not what you want.
    /// It means buttons will be displayed without borders or backgrounds, text inputs won't
    /// have default padding, etc.
    /// You should either add a thorough set of values to this, or start from an existing map.
    pub fn new() -> Self {
        Self {
            map: HashMap::new(),
            dummy_map: AnyMap::new(),
        }
    }

    /// Set the default value of property `P` for widget `W`.
    ///
    /// Widgets for which the property `P` isn't set will get `value` instead.
    pub fn insert<W: Widget, P: Property>(&mut self, value: P) -> Option<P> {
        self.map.entry(TypeId::of::<W>()).or_default().insert(value)
    }

    pub(crate) fn for_widget(&self, id: TypeId) -> &AnyMap {
        self.map.get(&id).unwrap_or(&self.dummy_map)
    }
}
