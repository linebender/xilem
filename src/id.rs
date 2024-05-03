// Copyright 2022 the Xilem Authors and the Druid Authors
// SPDX-License-Identifier: Apache-2.0

use std::num::NonZeroU64;

#[derive(Clone, Copy, Debug, PartialEq, Eq, PartialOrd, Hash)]
/// A stable identifier for an element.
pub struct Id(NonZeroU64);

impl Id {
    /// Allocate a new, unique `Id`.
    pub fn next() -> Id {
        use std::sync::atomic::{AtomicU64, Ordering};
        static WIDGET_ID_COUNTER: AtomicU64 = AtomicU64::new(1);
        Id(WIDGET_ID_COUNTER
            .fetch_add(1, Ordering::Relaxed)
            .try_into()
            .unwrap())
    }

    pub fn to_raw(self) -> u64 {
        self.0.into()
    }

    pub fn to_nonzero_raw(self) -> NonZeroU64 {
        self.0
    }
}
