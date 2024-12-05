// Copyright 2024 the Xilem Authors
// SPDX-License-Identifier: Apache-2.0

//! This crate implements a tree data structure for use in Masonry
//! It contains both a safe implementation (that is used by default)
//! and an unsafe implementation that can be used to improve performance
//!
//! The safe version is the first class citizen
//!
//! * The safe version may have features / APIs that the unsafe version doesn't yet have.
//! * If both versions are at feature parity, Masonry can switch on the unsafe version for best performance.
//! * Otherwise, Masonry uses the safe version.
type NodeId = u64;

#[cfg(not(feature = "safe_tree"))]
mod tree_arena_unsafe;
#[cfg(not(feature = "safe_tree"))]
pub use tree_arena_unsafe::*;

#[cfg(feature = "safe_tree")]
mod tree_arena_safe;
#[cfg(feature = "safe_tree")]
pub use tree_arena_safe::*;
