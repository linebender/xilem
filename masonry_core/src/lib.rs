// Copyright 2025 the Xilem Authors
// SPDX-License-Identifier: Apache-2.0

// cargo rdme --workspace-project=masonry_core
// After editing the below, then check links in README.md

//! Traits and types of the Masonry toolkit.
//! See [Masonry's documentation] for more details, examples and resources.
//!
//! [Masonry's documentation]: https://docs.rs/masonry/latest/

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
#![cfg_attr(
    test,
    expect(
        unused_crate_dependencies,
        reason = "False-positive with dev-dependencies only used in examples"
    )
)]
// TODO: Remove any items listed as "Deferred"
#![cfg_attr(not(debug_assertions), expect(unused, reason = "Deferred: Noisy"))]
#![expect(missing_debug_implementations, reason = "Deferred: Noisy")]
#![expect(clippy::cast_possible_truncation, reason = "Deferred: Noisy")]
// TODO - Add logo

pub use anymore;
pub use vello::{kurbo, peniko, peniko::color::palette};
pub use {accesskit, dpi, parley, ui_events, vello};

// TODO - re-add #[doc(hidden)]
pub mod doc;

#[macro_use]
pub mod util;

mod passes;

pub mod app;
pub mod core;
