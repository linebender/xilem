// Copyright 2026 the Xilem Authors
// SPDX-License-Identifier: Apache-2.0

use crate::core::Property;

/// Declares if the scroll bar collapses when not being hovered.
#[derive(Default, Clone, Copy, Debug, PartialEq)]
pub struct AutoHideScrollBar(pub bool);

impl Property for AutoHideScrollBar {
    fn static_default() -> &'static Self {
        static DEFAULT: AutoHideScrollBar = AutoHideScrollBar(false);
        &DEFAULT
    }
}
