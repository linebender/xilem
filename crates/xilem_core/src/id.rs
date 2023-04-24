// Copyright 2022 The Druid Authors.
// SPDX-License-Identifier: Apache-2.0

use std::{
    num::NonZeroU64,
    sync::atomic::{AtomicU64, Ordering},
};

#[derive(Clone, Copy, Debug, PartialEq, Eq, PartialOrd, Hash)]
/// A stable identifier for an element.
pub struct Id(NonZeroU64);

pub type IdPath = Vec<Id>;

impl Id {
    /// Allocate a new, unique `Id`.
    pub fn next() -> Id {
        static ID_COUNTER: AtomicU64 = AtomicU64::new(1);
        // Note: we can make the safety argument for the unchecked version.
        Id(NonZeroU64::new(ID_COUNTER.fetch_add(1, Ordering::Relaxed)).unwrap())
    }

    #[allow(unused)]
    pub fn to_raw(self) -> u64 {
        self.0.into()
    }

    pub fn to_nonzero_raw(self) -> NonZeroU64 {
        self.0
    }

    /*
    /// Turns an `accesskit::NodeId` id into an `Id`.
    ///
    /// This method will only return `Some` for `accesskit::NodeId` values which were created from
    /// `Id`'s.
    ///
    // TODO: Maybe we should not use AccessKit Ids at all in Widget implementation and do the
    //  mapping in the `App`.
    pub fn try_from_accesskit(id: accesskit::NodeId) -> Option<Self> {
        id.0.try_into().ok().map(|id| Id(id))
    }
    */
}

// Discussion question: do we need AccessKit integration for id's at the view level, or is
// that primarily a widget concern? If the former, then we should probably have a feature
// that enables these conversions.

/*
impl From<Id> for accesskit::NodeId {
    fn from(id: Id) -> accesskit::NodeId {
        id.to_nonzero_raw().into()
    }
}
*/
