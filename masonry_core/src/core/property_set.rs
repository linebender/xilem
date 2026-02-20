// Copyright 2026 the Xilem Authors
// SPDX-License-Identifier: Apache-2.0

use std::default::Default;

use crate::core::Property;
use crate::util::AnyMap;

// TODO - Implement Debug.
/// A collection of [properties](Property) that a widget can be created with.
#[derive(Clone, Default)]
pub struct PropertySet {
    pub(crate) map: AnyMap,
}

impl PropertySet {
    /// Creates an empty collection of properties.
    pub fn new() -> Self {
        Self { map: AnyMap::new() }
    }

    /// Creates a collection with a single property.
    pub fn one<P: Property>(value: P) -> Self {
        Self::new().with(value)
    }

    /// Builder-style method to add a property `P` with the given value.
    ///
    /// If the value was already set, it's discarded and replaced with the new value.
    pub fn with<P: Property>(mut self, value: P) -> Self {
        self.map.insert(value);
        self
    }

    /// Returns value of property `P`.
    pub fn get<P: Property>(&self) -> Option<&P> {
        self.map.get::<P>()
    }

    /// Sets property `P` to given value. Returns the previous value if `P` was already set.
    pub fn insert<P: Property>(&mut self, value: P) -> Option<P> {
        self.map.insert(value)
    }

    /// Removes property `P`. Returns the previous value if `P` was set.
    pub fn remove<P: Property>(&mut self) -> Option<P> {
        self.map.remove::<P>()
    }
}

impl<P: Property> From<P> for PropertySet {
    fn from(prop: P) -> Self {
        Self::one(prop)
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
        From<( $($Type,)* )> for PropertySet
        {
            fn from(value: ( $($Type,)* )) -> Self {
                PropertySet::new()
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
