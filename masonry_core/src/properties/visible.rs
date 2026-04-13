// Copyright 2026 the Xilem Authors
// SPDX-License-Identifier: Apache-2.0

use crate::core::{HasProperty, Property, Widget};

// Every widget may be invisible.
impl<W: Widget> HasProperty<Visible> for W {}

/// The visibility of a widget.
///
/// This property lets you skip a widget and its children from the paint tree.
#[expect(missing_docs, reason = "field names are self-descriptive")]
#[derive(Clone, Copy, Debug, PartialEq)]
pub struct Visible {
    pub value: bool,
}

// ---

impl Property for Visible {
    fn static_default() -> &'static Self {
        static DEFAULT: Visible = Visible { value: true };
        &DEFAULT
    }
}

impl Default for Visible {
    fn default() -> Self {
        *Self::static_default()
    }
}

impl Visible {
    /// Creates new `Visible` with given value.
    pub const fn new(visible: bool) -> Self {
        Self { value: visible }
    }
}
