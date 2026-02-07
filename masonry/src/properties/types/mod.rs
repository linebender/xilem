// Copyright 2025 the Xilem Authors
// SPDX-License-Identifier: Apache-2.0

//! Value types used in properties.
//!
//! The types in this modules aren't properties themselves: they're types that the fields
//! and variants of properties can have.
//!
//! So for instance, you can't set a button's [`Gradient`], but you can set a button's
//! [`Background`] to [`Background::Gradient`], which takes a `Gradient` value.
//!
//! [`Background`]: crate::properties::Background
//! [`Background::Gradient`]: crate::properties::Background::Gradient

mod alignment;

pub use alignment::*;

pub use masonry_core::properties::types::*;
