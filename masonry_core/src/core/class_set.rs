// Copyright 2026 the Xilem Authors
// SPDX-License-Identifier: Apache-2.0

use std::collections::HashSet;

/// A set of classes and pseudo-classes that can be used for styling widgets.
#[derive(Clone, Debug, Default, PartialEq, Eq)]
pub(crate) struct ClassSet {
    pub(crate) classes: HashSet<String>,
    pub(crate) is_hovered: bool,
    pub(crate) is_active: bool,
    pub(crate) is_disabled: bool,
    pub(crate) has_focus_target: bool,
}

/// A series of changes that need to be applied to a [`ClassSet`] during the next
/// update_props pass.
#[derive(Clone, Debug, Default, PartialEq, Eq)]
pub(crate) struct ClassSetDiff {
    // TODO - Use Hashmap of enums instead?
    pub(crate) added: HashSet<String>,
    pub(crate) removed: HashSet<String>,
    // None = no change; Some(v) = set this flag to v
    pub(crate) is_hovered: Option<bool>,
    pub(crate) is_active: Option<bool>,
    pub(crate) is_disabled: Option<bool>,
    pub(crate) has_focus_target: Option<bool>,
}

// ---

impl ClassSet {
    pub(crate) fn add_class(&mut self, class: &str) {
        self.classes.insert(class.to_string());
    }

    pub(crate) fn apply(&mut self, diff: &ClassSetDiff) {
        for class in &diff.added {
            self.add_class(class);
        }
        for class in &diff.removed {
            self.classes.remove(class);
        }
        if let Some(v) = diff.is_hovered {
            self.is_hovered = v;
        }
        if let Some(v) = diff.is_active {
            self.is_active = v;
        }
        if let Some(v) = diff.is_disabled {
            self.is_disabled = v;
        }
        if let Some(v) = diff.has_focus_target {
            self.has_focus_target = v;
        }
    }
}

// ---

impl ClassSetDiff {
    pub(crate) fn add(&mut self, class: &str) {
        self.removed.remove(class);
        self.added.insert(class.to_string());
    }

    pub(crate) fn remove(&mut self, class: &str) {
        self.added.remove(class);
        self.removed.insert(class.to_string());
    }
}
