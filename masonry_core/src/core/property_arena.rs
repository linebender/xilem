// Copyright 2026 the Xilem Authors
// SPDX-License-Identifier: Apache-2.0

use std::collections::HashMap;

use crate::core::{PropertyStack, PropertyStackId};

/// An arena for storing [`PropertyStack`]s, which represent cascading sets of properties that can be applied to widgets.
#[derive(Default)]
pub struct PropertyArena {
    pub(crate) arena: HashMap<PropertyStackId, PropertyStack>,
}

impl PropertyArena {
    /// Creates an empty `PropertyArena`.
    pub fn new() -> Self {
        Self::default()
    }

    /// Inserts a `PropertyStack` into the arena and returns its unique `PropertyStackId`.
    pub fn insert(&mut self, stack: PropertyStack) -> PropertyStackId {
        let id = PropertyStackId::next();
        self.arena.insert(id, stack);
        id
    }

    /// Retrieves a reference to a `PropertyStack` by its `PropertyStackId`, or `None` if not found.
    pub fn get(&self, id: Option<PropertyStackId>) -> Option<&PropertyStack> {
        self.arena.get(&id?)
    }
}
