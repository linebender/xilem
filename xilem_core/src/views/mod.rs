// Copyright 2024 the Xilem Authors
// SPDX-License-Identifier: Apache-2.0

mod memoize;
pub use memoize::{memoize, Memoize};

#[path = "one_of.rs"]
mod one_of_;
/// Statically typed alternatives to the type-erased [`crate::AnyView`].
pub mod one_of {
    pub use super::one_of_::{
        OneOf2, OneOf2Ctx, OneOf3, OneOf3Ctx, OneOf4, OneOf4Ctx, OneOf5, OneOf5Ctx, OneOf6,
        OneOf6Ctx, OneOf7, OneOf7Ctx, OneOf8, OneOf8Ctx, OneOf9, OneOf9Ctx,
    };
}
