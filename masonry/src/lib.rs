// Copyright 2018 the Xilem Authors and the Druid Authors
// SPDX-License-Identifier: Apache-2.0

//! Traits and types of the Masonry toolkit.
//! See [Masonry Winit's documentation] for more details, examples and resources.
//!
//! [Masonry Winit's documentation]: https://docs.rs/masonry_winit/latest/

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
#![cfg_attr(
    test,
    expect(
        unused_crate_dependencies,
        reason = "False-positive with dev-dependencies only used in examples"
    )
)]
#![expect(clippy::allow_attributes_without_reason, reason = "Deferred: Noisy")]
// TODO: Remove any items listed as "Deferred"
#![expect(clippy::should_implement_trait, reason = "Deferred: Noisy")]
#![cfg_attr(not(debug_assertions), expect(unused, reason = "Deferred: Noisy"))]
#![expect(missing_debug_implementations, reason = "Deferred: Noisy")]
#![expect(clippy::cast_possible_truncation, reason = "Deferred: Noisy")]
#![expect(clippy::missing_assert_message, reason = "Deferred: Noisy")]
#![expect(elided_lifetimes_in_paths, reason = "Deferred: Noisy")]
#![expect(unreachable_pub, reason = "Potentially controversial code style")]
#![expect(
    unnameable_types,
    reason = "Requires lint_reasons rustc feature for exceptions"
)]
#![expect(clippy::todo, reason = "We have a lot of 'real' todos")]
#![expect(clippy::single_match, reason = "General policy not decided")]

// TODO - Add logo

#[macro_use]
pub mod util;

// TODO - re-add #[doc(hidden)]
pub mod doc;

mod passes;

pub mod app;
pub mod core;
pub mod properties;
pub mod testing;
pub mod theme;
pub mod widgets;

pub use vello::peniko::color::palette;
pub use vello::{kurbo, peniko};
pub use {cursor_icon, dpi, parley, vello};

// TODO - Move to core?
pub use util::{Handled, UnitPoint};
