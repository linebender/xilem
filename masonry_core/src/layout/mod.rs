// Copyright 2025 the Xilem Authors
// SPDX-License-Identifier: Apache-2.0

//! Core layout types and traits Masonry is built on.

mod as_unit;
mod dim;
mod layout_size;
mod len_def;
mod len_req;
mod length;
mod measurement_cache;
mod size_def;

pub use as_unit::*;
pub use dim::*;
pub use layout_size::*;
pub use len_def::*;
pub use len_req::*;
pub use length::*;
pub(crate) use measurement_cache::*;
pub use size_def::*;

use vello::kurbo::Size;

/// Ergonomic layout helper methods for [`Size`].
pub trait LayoutCalc {
    /// Reduces the width by the given `delta`.
    ///
    /// The width is clamped to zero.
    fn sub_width(self, delta: f64) -> Self;

    /// Returns the height by the given `delta`.
    ///
    /// The height is clamped to zero.
    fn sub_height(self, delta: f64) -> Self;
}

impl LayoutCalc for Size {
    fn sub_width(mut self, delta: f64) -> Self {
        self.width = (self.width - delta).max(0.);
        self
    }

    fn sub_height(mut self, delta: f64) -> Self {
        self.height = (self.height - delta).max(0.);
        self
    }
}
