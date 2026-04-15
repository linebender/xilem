// Copyright 2024 the Xilem Authors
// SPDX-License-Identifier: Apache-2.0

// After you edit the crate's doc comment, run this command, then check README.md for any missing links
// cargo rdme --workspace-project=xilem_masonry

//! An implementation of the Xilem architecture (through [Xilem Core][]) using [Masonry][] widgets as Xilem elements.
//!
//! You probably shouldn't depend on this crate directly, unless you're trying to embed Xilem into a non-Winit platform.
//! See [Xilem][] or [Xilem Web][] instead.
//!
//! [Xilem Core]: xilem_core
//! [Masonry]: masonry
//! [Xilem]: https://github.com/linebender/xilem/tree/main/xilem
//! [Xilem Web]: https://github.com/linebender/xilem/tree/main/xilem_web

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
#![cfg_attr(docsrs, feature(doc_cfg))]
// TODO: Remove any items listed as "Deferred"
#![expect(
    missing_debug_implementations,
    reason = "Deferred: Noisy. Requires same lint to be addressed in Masonry"
)]
#![expect(clippy::missing_assert_message, reason = "Deferred: Noisy")]

pub use masonry;
pub use xilem_core as core;

pub mod style;
pub mod view;

mod any_view;
mod masonry_root;
mod one_of;
mod pod;
mod view_ctx;
mod widget_view;

pub use any_view::AnyWidgetView;
pub use masonry_root::{InitialRootWidget, MasonryRoot};
pub use pod::Pod;
pub use view_ctx::ViewCtx;
pub use widget_view::{WidgetView, WidgetViewSequence};

// TODO - Remove these re-exports and fix the places in the crate that use them
pub(crate) use masonry::parley::Alignment as TextAlign;
pub(crate) use masonry::peniko::Color;
pub(crate) use masonry::widgets::InsertNewline;
