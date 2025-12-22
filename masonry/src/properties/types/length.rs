// Copyright 2025 the Xilem Authors
// SPDX-License-Identifier: Apache-2.0

use masonry_core::debug_panic;

/// A value representing a width, height, or similar distance value.
///
/// Its value is always finite and non-negative.
#[derive(Clone, Copy, PartialEq)]
pub struct Length {
    value: f64,
}

impl std::fmt::Debug for Length {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        write!(f, "{self}")
    }
}

impl std::fmt::Display for Length {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        write!(f, "{}px", self.value)
    }
}

impl Length {
    /// A length of zero.
    pub const ZERO: Self = Self { value: 0. };

    /// Creates a length, in logical pixels.
    ///
    /// # Panics
    ///
    /// If debug assertions are on, this will panic in these cases:
    ///
    /// - `value` is NaN.
    /// - `value` is infinite.
    /// - `value` is negative.
    ///
    /// If debug assertions are off, this will return zero instead of panicking.
    #[track_caller]
    pub fn px(value: f64) -> Self {
        if value < 0. || !value.is_finite() {
            // TODO - Make const once const formatting is allowed.
            // (aka see you in 2030)
            debug_panic!("Invalid length value '{value}'");
            return Self::ZERO;
        }
        Self { value }
    }

    /// Creates a length, in logical pixels.
    ///
    /// Can be called from const contexts.
    ///
    /// # Panics
    ///
    /// This will always panic if value is negative or non-finite.
    #[track_caller]
    pub const fn const_px(value: f64) -> Self {
        if value < 0. || !value.is_finite() {
            panic!("Invalid length value");
        }
        Self { value }
    }

    /// Returns the value, in logical pixels.
    pub const fn get(self) -> f64 {
        self.value
    }
}
