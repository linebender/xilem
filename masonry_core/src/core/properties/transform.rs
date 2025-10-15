// Copyright 2025 the Xilem Authors
// SPDX-License-Identifier: Apache-2.0

use vello::kurbo::Affine;

use crate::core::{GlobalProperty, Property};

/// The local transform of a widget
#[derive(Clone, Copy, Default, Debug, PartialEq)]
pub struct Transform {
    /// Local transform of the widget
    pub transform: Affine,
}

impl Property for Transform {
    fn static_default() -> &'static Self {
        static DEFAULT: Transform = Transform {
            transform: Affine::IDENTITY,
        };
        &DEFAULT
    }
}

impl GlobalProperty for Transform {}

impl From<Affine> for Transform {
    fn from(transform: Affine) -> Self {
        Self { transform }
    }
}
