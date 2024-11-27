// Copyright 2024 the Xilem Authors
// SPDX-License-Identifier: Apache-2.0

//! This crate implements a tree data structure for use in Masonry
//! It contains both a safe implementation (that is used by default)
//! and an unsafe implementation that can be used to improve performance
type NodeId = u64;

#[cfg(not(feature = "safe_tree"))]
mod tree_arena_unsafe;
#[cfg(not(feature = "safe_tree"))]
pub use tree_arena_unsafe::*;

#[cfg(feature = "safe_tree")]
mod tree_arena_safe;
#[cfg(feature = "safe_tree")]
pub use tree_arena_safe::*;
