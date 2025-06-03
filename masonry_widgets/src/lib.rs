// Copyright 2025 the Xilem Authors
// SPDX-License-Identifier: Apache-2.0

//! The default widget and property set for the Masonry toolkit.
//!
//! By default, Masonry only provides a GUI engine with traits and some general-purpose types.
//! This crate is the one that provides Buttons, Checkboxes, Flex containers, etc, and the
//! styling options for those widgets.
//!
//! See [Masonry Winit's documentation] for more details, examples and resources.
//!
//! [Masonry Winit's documentation]: https://docs.rs/masonry_winit/latest/

#![expect(missing_debug_implementations, reason = "Deferred: Noisy")]
#![expect(clippy::cast_possible_truncation, reason = "Deferred: Noisy")]
#![expect(clippy::missing_assert_message, reason = "Deferred: Noisy")]
#![expect(elided_lifetimes_in_paths, reason = "Deferred: Noisy")]
#![expect(unreachable_pub, reason = "Potentially controversial code style")]
#![expect(clippy::single_match, reason = "General policy not decided")]
#![expect(clippy::allow_attributes_without_reason, reason = "Deferred: Noisy")]

pub use vello::peniko::color::palette;

pub mod properties;
pub mod theme;
pub mod widgets;

// TODO - Remove these re-exports
pub(crate) use masonry::core;
pub(crate) use masonry::include_screenshot;
pub(crate) use masonry::kurbo;
pub(crate) use masonry::peniko;
pub(crate) use masonry::util;
#[cfg(test)]
pub(crate) use masonry_testing as testing;
#[cfg(test)]
pub(crate) use masonry_testing::assert_render_snapshot;
