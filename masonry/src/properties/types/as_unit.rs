// Copyright 2025 the Xilem Authors
// SPDX-License-Identifier: Apache-2.0

use crate::properties::types::Length;

// NOTE ON OVERFLOW:
// Casting a u64 to f64 can never overflow. In some cases, we may lose precision.
// This is mostly fine, given that lengths large enough to overflow an f64 will be extremely rare.
// See reference for details on casts:
// https://doc.rust-lang.org/reference/expressions/operator-expr.html#r-expr.as.numeric.int-as-float

/// Utility trait for wrapping numbers in logical units.
pub trait AsUnit {
    /// Create a length, in logical pixels.
    ///
    /// # Panics
    ///
    /// Panics if value is negative, infinite.
    #[track_caller]
    fn px(self) -> Length;
}

impl AsUnit for f64 {
    #[track_caller]
    fn px(self) -> Length {
        Length::px(self)
    }
}

impl AsUnit for f32 {
    #[track_caller]
    fn px(self) -> Length {
        Length::px(self as f64)
    }
}

impl AsUnit for u64 {
    #[track_caller]
    fn px(self) -> Length {
        Length::px(self as f64)
    }
}

impl AsUnit for u32 {
    #[track_caller]
    fn px(self) -> Length {
        Length::px(self as f64)
    }
}

impl AsUnit for u16 {
    #[track_caller]
    fn px(self) -> Length {
        Length::px(self as f64)
    }
}

impl AsUnit for u8 {
    #[track_caller]
    fn px(self) -> Length {
        Length::px(self as f64)
    }
}

impl AsUnit for i64 {
    #[track_caller]
    fn px(self) -> Length {
        Length::px(self as f64)
    }
}

impl AsUnit for i32 {
    #[track_caller]
    fn px(self) -> Length {
        Length::px(self as f64)
    }
}

impl AsUnit for i16 {
    #[track_caller]
    fn px(self) -> Length {
        Length::px(self as f64)
    }
}

impl AsUnit for i8 {
    #[track_caller]
    fn px(self) -> Length {
        Length::px(self as f64)
    }
}
