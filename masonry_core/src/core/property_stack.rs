// Copyright 2026 the Xilem Authors
// SPDX-License-Identifier: Apache-2.0

use std::fmt::Display;
use std::num::NonZeroU64;
use std::sync::atomic::{AtomicU64, Ordering};

use crate::core::{PropertySet, Selector};

/// A unique identifier for a single [`PropertyStack`].
#[derive(Clone, Copy, Debug, Hash, PartialEq, Eq)]
pub struct PropertyStackId(pub(crate) NonZeroU64);

/// TODO - Placeholder type for future PR.
#[derive(Default)]
#[expect(dead_code, reason = "Future PR")]
pub struct PropertyStack {
    pub(crate) stack: Vec<(Selector, PropertySet)>,
}

// ---

impl PropertyStackId {
    /// Allocates a new, unique `PropertyStackId`.
    pub fn next() -> Self {
        static PROPERTY_STACK_ID_COUNTER: AtomicU64 = AtomicU64::new(1);
        let id = PROPERTY_STACK_ID_COUNTER.fetch_add(1, Ordering::Relaxed);
        Self(id.try_into().unwrap())
    }

    /// Returns the integer value of the `PropertyStackId`.
    pub fn to_raw(self) -> u64 {
        self.0.into()
    }
}

impl From<PropertyStackId> for u64 {
    fn from(id: PropertyStackId) -> Self {
        id.0.into()
    }
}

impl Display for PropertyStackId {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        write!(f, "#{}", self.0)
    }
}

// ---

impl PropertyStack {
    /// Creates an empty `PropertyStack`.
    pub const fn new() -> Self {
        Self { stack: Vec::new() }
    }
}
