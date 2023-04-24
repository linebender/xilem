// Copyright 2023 the Druid Authors.
// SPDX-License-Identifier: Apache-2.0

use bitflags::bitflags;

bitflags! {
    #[derive(Default)]
    #[must_use]
    pub struct ChangeFlags: u8 {
        const UPDATE = 1;
        const LAYOUT = 2;
        const ACCESSIBILITY = 4;
        const PAINT = 8;
        const TREE = 0x10;
        const DESCENDANT_REQUESTED_ACCESSIBILITY = 0x20;
    }
}

impl ChangeFlags {
    pub fn upwards(self) -> Self {
        let mut result = self;
        if self.contains(ChangeFlags::ACCESSIBILITY) {
            result.remove(ChangeFlags::ACCESSIBILITY);
            result.insert(ChangeFlags::DESCENDANT_REQUESTED_ACCESSIBILITY);
        }
        result
    }
}
