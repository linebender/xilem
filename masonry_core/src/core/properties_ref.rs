// Copyright 2026 the Xilem Authors
// SPDX-License-Identifier: Apache-2.0

use crate::core::Property;
use crate::util::AnyMap;

/// Reference to a collection of [properties](Property) that a widget has access to.
///
/// Used by the [`Widget`] trait during rendering passes and in some search methods.
#[derive(Clone, Copy)]
pub struct PropertiesRef<'a> {
    pub(crate) map: &'a AnyMap,
    pub(crate) default_map: &'a AnyMap,
}

// TODO - Better document local vs default properties.

impl PropertiesRef<'_> {
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
}
