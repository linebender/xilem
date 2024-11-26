// Copyright 2024 the Xilem Authors
// SPDX-License-Identifier: Apache-2.0

//! This module will eventually be factored out into a separate crate.
//!
//! In the meantime, we intentionally don't make the types in this module part of
//! our public API, but still implement methods that a standalone crate would have.
//!
//! The types defined in this module don't *actually* implement an arena. They use
//! 100% safe code, which has a significant performance overhead. The final version
//! will use an arena and unsafe code, but should have the exact same exported API as
//! this module.

type NodeId = u64;

#[cfg(not(feature = "safe_tree"))]
mod tree_arena_unsafe;
#[cfg(not(feature = "safe_tree"))]
pub use tree_arena_unsafe::*;

#[cfg(feature = "safe_tree")]
mod tree_arena_safe;
#[cfg(feature = "safe_tree")]
pub use tree_arena_safe::*;
