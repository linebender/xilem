// Copyright 2026 the Xilem Authors
// SPDX-License-Identifier: Apache-2.0

use std::any::TypeId;
use std::collections::HashMap;
use std::default::Default;

use crate::core::{Property, Widget};
use crate::util::AnyMap;

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

impl DefaultProperties {
    /// Creates an empty property map with no default values.
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

    /// Sets the default value of property `P` for widget `W`.
    ///
    /// Widgets for which the property `P` isn't set will get `value` instead.
    pub fn insert<W: Widget, P: Property>(&mut self, value: P) -> Option<P> {
        self.map.entry(TypeId::of::<W>()).or_default().insert(value)
    }

    pub(crate) fn for_widget(&self, id: TypeId) -> &AnyMap {
        self.map.get(&id).unwrap_or(&self.dummy_map)
    }
}
