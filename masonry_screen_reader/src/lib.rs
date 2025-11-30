// Copyright 2025 the Xilem Authors
// SPDX-License-Identifier: Apache-2.0

//! Screen reader debug output for Masonry accessibility trees.
//!
//! This crate provides debugging tools to simulate screen reader behavior by generating
//! human-readable descriptions of accessibility tree updates. It helps developers understand
//! what information would be announced to screen reader users as they interact with a Masonry
//! application.

mod adapter;
mod filter;

pub use adapter::ScreenReader;
