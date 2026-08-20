// Copyright 2026 the Xilem Authors
// SPDX-License-Identifier: Apache-2.0

use crate::core::Property;

/// The duration of an animation.
#[expect(missing_docs, reason = "field names are self-descriptive")]
#[derive(Clone, Copy, Debug, PartialEq)]
pub struct AnimationDuration {
    pub seconds: f64,
}

impl Default for AnimationDuration {
    fn default() -> Self {
        *Self::static_default()
    }
}

impl Property for AnimationDuration {
    fn static_default() -> &'static Self {
        static DEFAULT: AnimationDuration = AnimationDuration { seconds: 1.0 };
        &DEFAULT
    }

    fn matches(property_type: std::any::TypeId) -> bool {
        property_type == std::any::TypeId::of::<Self>()
    }
}
