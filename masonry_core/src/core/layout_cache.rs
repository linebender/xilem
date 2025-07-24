// Copyright 2025 the Xilem Authors
// SPDX-License-Identifier: Apache-2.0

use crate::core::BoxConstraints;

/// A struct that holds everything Masonry might want to cache per-widget between layout passes.
#[derive(Clone, Debug)]
pub(crate) struct LayoutCache {
    pub(crate) old_bc: Option<BoxConstraints>,
}

impl LayoutCache {
    pub(crate) fn empty() -> Self {
        Self { old_bc: None }
    }
}
