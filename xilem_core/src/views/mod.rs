// Copyright 2024 the Xilem Authors
// SPDX-License-Identifier: Apache-2.0

mod memoize;
pub use memoize::{memoize, Memoize};

/// Statically typed alternatives to the type-erased [`AnyView`](`crate::AnyView`).
pub mod one_of;