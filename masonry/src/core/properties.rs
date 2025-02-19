// Copyright 2025 the Xilem Authors
// SPDX-License-Identifier: Apache-2.0

#![allow(missing_docs, reason = "TODO")]

use anymap3::{AnyMap, Entry};

pub struct Properties {
    pub(crate) map: AnyMap,
}

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
    pub fn get<T: 'static>(&self) -> Option<&T> {
        self.map.get::<T>()
    }

    pub fn contains<T: 'static>(&self) -> bool {
        self.map.contains::<T>()
    }
}

impl PropertiesMut<'_> {
    pub fn get<T: 'static>(&self) -> Option<&T> {
        self.map.get::<T>()
    }

    pub fn contains<T: 'static>(&self) -> bool {
        self.map.contains::<T>()
    }

    pub fn get_mut<T: 'static>(&mut self) -> Option<&mut T> {
        self.map.get_mut::<T>()
    }

    pub fn insert<T: 'static>(&mut self, value: T) -> Option<T> {
        self.map.insert(value)
    }

    pub fn remove<T: 'static>(&mut self) -> Option<T> {
        self.map.remove::<T>()
    }

    pub fn entry<T: 'static>(&mut self) -> Entry<'_, dyn std::any::Any, T> {
        self.map.entry::<T>()
    }
}
