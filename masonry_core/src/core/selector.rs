// Copyright 2026 the Xilem Authors
// SPDX-License-Identifier: Apache-2.0

/// A predicate for matching widgets based on their classes and pseudo-classes.
///
/// This type is mostly used for property resolution.
///
/// TODO - Placeholder type for future PR.
#[derive(Clone, Debug, Default)]
pub struct Selector {
    // TODO
}

impl Selector {
    /// Creates an empty `Selector` that matches all widgets.
    pub fn new() -> Self {
        Self::default()
    }
}
