// Copyright 2025 the Xilem Authors
// SPDX-License-Identifier: Apache-2.0

//! This module imports widget code from most of the examples.
//!
//! When running integration tests, it will compare the rendered snapshots of the widgets
//! with the expected snapshots.

#[cfg(test)]
#[allow(dead_code, reason = "We don't need to run the main functions.")]
#[allow(missing_docs, reason = "Example code doesn't need docs.")]
#[path = "../examples"]
pub mod others {
    pub mod calc_masonry;
    pub mod custom_widget;
    pub mod grid_masonry;
    pub mod simple_image;
    pub mod to_do_list;
}
