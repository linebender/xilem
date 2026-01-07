// Copyright 2025 the Xilem Authors
// SPDX-License-Identifier: Apache-2.0

// After you edit the crate's doc comment, run this command, then check README.md for any missing links
// cargo rdme --workspace-project=masonry_core

#![doc(
    html_logo_url = "https://raw.githubusercontent.com/linebender/xilem/main/docs/assets/masonry-logo.svg"
)]

//! Masonry Core provides the base GUI engine for Masonry.
//!
//! Masonry's widgets are implemented in the Masonry crate, which re-exports this crate as `masonry::core`.
//! Most users who wish to use Masonry for creating applications (and UI libraries) should
//! prefer to depend on Masonry directly (i.e. the `masonry` crate).
//! [Masonry's documentation] can be found on docs.rs.
//!
//! Masonry Core provides:
//!
//! - [`Widget`][core::Widget], the trait for GUI widgets in Masonry.
//! - Event handling and bubbling, using types from [`ui-events`][ui_events] for interoperability.
//! - Communication between parent and child widgets for layout.
//! - Compositing of widget's content (to be rendered using [Vello][vello]).
//! - Creation of accessibility trees using [Accesskit][accesskit].
//! - APIs for widget manipulation (such as [`WidgetMut`][core::WidgetMut]).
//! - The [`Action`][core::Widget::Action] mechanism by which widgets send events to the application.
//!
//! Details of many of these can be found in the [Pass System][doc::pass_system] article.
//!
//! If you're writing a library in the Masonry ecosystem, you should depend on `masonry_core`
//! directly where possible (instead of depending on `masonry`).
//! This will allow applications using your library to have greater compilation parallelism.
//! Cases where this apply include:
//!
//! - Writing an alternative driver for Masonry (alike to [Masonry Winit][]).
//! - Witing a library containing one or more custom widget (such as a 2d mapping widget).
//!
//! Masonry Core can also be used by applications wishing to not use Masonry's provided
//! set of widgets, so as to have more control.
//! This can be especially useful if you wish to exactly match the appearance of an existing library,
//! or enforce following a specific design guide, which Masonry's widgets may not always allow.
//! Masonry Core provides a useful shared set of functionality to implement alternative widget libraries.
//! Note that Masonry Core is currently focused primarily on the main Masonry crate itself, as we're
//! not aware of any projects using Masonry Core as described in this paragraph.
//!
//! # Feature flags
//!
//! The following crate [feature flags](https://doc.rust-lang.org/cargo/reference/features.html#dependency-features) are available:
//!
//! - `default`: Enables the default features of [Vello][vello].
//! - `tracy`: Enables creating output for the [Tracy](https://github.com/wolfpld/tracy) profiler using [`tracing-tracy`][tracing_tracy].
//!   This can be used by installing Tracy and connecting to a Masonry with this feature enabled.
//!
//! [Masonry's documentation]: https://docs.rs/masonry/latest/
//! [Masonry Winit]: https://docs.rs/masonry_winit/latest/
//! [tracing_tracy]: https://crates.io/crates/tracing-tracy

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

pub use vello::peniko::color::palette;
pub use vello::{kurbo, peniko};
pub use {accesskit, anymore, dpi, parley, ui_events, vello};

// TODO - re-add #[doc(hidden)]
pub mod doc;

#[macro_use]
pub mod util;

mod passes;

pub mod app;
pub mod core;
