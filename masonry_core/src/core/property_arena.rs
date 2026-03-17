// Copyright 2026 the Xilem Authors
// SPDX-License-Identifier: Apache-2.0

use std::{any::TypeId, collections::HashMap, sync::Arc};

use crate::core::{DefaultProperties, PropertyStack, PropertyStackId};

/// An arena for storing [`PropertyStack`]s, which represent cascading sets of properties that can be applied to widgets.
#[derive(Default)]
pub struct PropertyArena {
    pub(crate) arena: HashMap<PropertyStackId, PropertyStack>,

    /// Default values that properties will have if not defined per-widget.
    pub(crate) default_properties: Arc<DefaultProperties>,
}

impl PropertyArena {
    /// Creates an empty `PropertyArena` with the given default properties.
    pub fn new(default_properties: Arc<DefaultProperties>) -> Self {
        Self {
            arena: HashMap::new(),
            default_properties,
        }
    }

    /// Inserts a `PropertyStack` into the arena and returns its unique `PropertyStackId`.
    pub fn insert(&mut self, stack: PropertyStack) -> PropertyStackId {
        let id = PropertyStackId::next();
        self.arena.insert(id, stack);
        id
    }

    /// Retrieves a reference to a `PropertyStack` by its `PropertyStackId`, or `None` if not found.
    pub fn get(&self, id: Option<PropertyStackId>, widget_type: TypeId) -> &PropertyStack {
        if let Some(id) = id
            && let Some(stack) = self.arena.get(&id)
        {
            stack
        } else {
            self.default_properties.stack_for_widget(widget_type)
        }
    }
}
