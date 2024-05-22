// Copyright 2024 the Xilem Authors
// SPDX-License-Identifier: Apache-2.0

#[derive(Copy, Clone, Debug)]
/// An identifier for a subtree in a view hierarchy.
// TODO: also provide debugging information to give e.g. a useful stack trace?
pub struct ViewId(u64);

impl ViewId {
    pub fn new(raw: u64) -> Self {
        Self(raw)
    }

    pub fn routing_id(self) -> u64 {
        self.0
    }
}
