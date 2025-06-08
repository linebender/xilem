// Copyright 2024 the Xilem Authors
// SPDX-License-Identifier: Apache-2.0

//! `xilem_masonry` provides Xilem views for the Masonry backend.
//!
//! Xilem is a portable, native UI framework written in Rust.
//! See [the Xilem documentation](https://docs.rs/xilem/latest/xilem/)
//! for details.
//!
//! [Masonry](masonry) is a foundational library for writing native GUI frameworks.
//!
//! Xilem's architecture uses lightweight view objects, diffing them to provide minimal
//! updates to a retained UI.
//!
//! `xilem_masonry` uses Masonry's widget tree as the retained UI.
#![doc(html_logo_url = "https://avatars.githubusercontent.com/u/46134943?s=48&v=4")]
// LINEBENDER LINT SET - lib.rs - v3
// See https://linebender.org/wiki/canonical-lints/
// These lints shouldn't apply to examples or tests.
#![cfg_attr(not(test), warn(unused_crate_dependencies))]
// These lints shouldn't apply to examples.
#![warn(clippy::print_stdout, clippy::print_stderr)]
// Targeting e.g. 32-bit means structs containing usize can give false positives for 64-bit.
#![cfg_attr(target_pointer_width = "64", warn(clippy::trivially_copy_pass_by_ref))]
// END LINEBENDER LINT SET
#![cfg_attr(docsrs, feature(doc_auto_cfg))]
// TODO: Remove any items listed as "Deferred"
#![cfg_attr(not(debug_assertions), allow(unused))]
#![expect(
    missing_debug_implementations,
    reason = "Deferred: Noisy. Requires same lint to be addressed in Masonry"
)]
#![expect(clippy::missing_assert_message, reason = "Deferred: Noisy")]
#![expect(elided_lifetimes_in_paths, reason = "Deferred: Noisy")]
// https://github.com/rust-lang/rust/pull/130025
#![expect(clippy::allow_attributes_without_reason, reason = "Deferred: Noisy")]

pub use masonry::kurbo::{Affine, Vec2};
pub use masonry::parley::Alignment as TextAlignment;
pub use masonry::parley::style::FontWeight;
pub use masonry::peniko::{Blob, Color};
pub use masonry::widgets::{InsertNewline, LineBreaking};
pub use masonry::{dpi, palette};
pub use xilem_core as core;

/// Tokio is the async runner used with Xilem.
pub use tokio;

mod any_view;
mod one_of;
mod pod;
mod property_tuple;
mod view_ctx;
mod widget_view;

pub mod style;
pub mod view;

pub use any_view::AnyWidgetView;
pub use pod::Pod;
pub use property_tuple::PropertyTuple;
pub use view_ctx::ViewCtx;
pub use widget_view::{WidgetView, WidgetViewSequence};
