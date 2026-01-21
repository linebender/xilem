// Copyright 2025 the Xilem Authors
// SPDX-License-Identifier: Apache-2.0

use std::any::TypeId;
use std::fmt::Display;

use masonry_core::core::Property;

/// Helper property for tying a name to a widget.
#[derive(Default, Debug, PartialEq, Eq, PartialOrd, Ord)]
pub struct DebugName(pub String);

impl Property for DebugName {
    fn static_default() -> &'static Self {
        static DEFAULT: DebugName = DebugName(String::new());
        &DEFAULT
    }

    #[inline(always)]
    fn matches(property_type: TypeId) -> bool {
        property_type == TypeId::of::<Self>()
    }
}

impl Display for DebugName {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        f.write_str(&self.0)
    }
}
