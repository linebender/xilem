// Copyright 2026 the Xilem Authors
// SPDX-License-Identifier: Apache-2.0

use std::collections::HashSet;

use crate::core::ClassSet;

/// A predicate for matching widgets based on their classes and pseudo-classes.
///
/// This type is mostly used for property resolution.
#[derive(Clone, Debug, Default)]
pub struct Selector {
    pub(crate) classes: HashSet<String>,
    // None means "don't filter on this flag"
    pub(crate) is_hovered: Option<bool>,
    pub(crate) is_active: Option<bool>,
    pub(crate) is_disabled: Option<bool>,
    pub(crate) has_focus_target: Option<bool>,
}

impl Selector {
    /// Creates an empty `Selector` that matches all widgets.
    pub fn new() -> Self {
        Self::default()
    }

    /// Creates a `Selector` that matches widgets with all of the specified classes.
    pub fn classes(classes: &[&str]) -> Self {
        let mut selector = Self::new();
        for class in classes {
            selector.classes.insert(class.to_string());
        }
        selector
    }

    /// Builder method for a selector that matches widgets with a specific "hovered" status.
    pub fn with_hovered(mut self, value: bool) -> Self {
        self.is_hovered = Some(value);
        self
    }

    /// Builder method for a selector that matches widgets with a specific "active" status.
    pub fn with_active(mut self, value: bool) -> Self {
        self.is_active = Some(value);
        self
    }

    /// Builder method for a selector that matches widgets with a specific "disabled" status.
    pub fn with_disabled(mut self, value: bool) -> Self {
        self.is_disabled = Some(value);
        self
    }

    /// Builder method for a selector that matches widgets with a specific "focused" status.
    pub fn with_focused(mut self, value: bool) -> Self {
        self.has_focus_target = Some(value);
        self
    }

    /// Checks whether this selector matches a given `ClassSet`.
    pub(crate) fn matches(&self, class_set: &ClassSet) -> bool {
        self.classes.is_subset(&class_set.classes)
            && self.is_hovered.is_none_or(|v| class_set.is_hovered == v)
            && self.is_active.is_none_or(|v| class_set.is_active == v)
            && self.is_disabled.is_none_or(|v| class_set.is_disabled == v)
            && self
                .has_focus_target
                .is_none_or(|v| class_set.has_focus_target == v)
    }
}
