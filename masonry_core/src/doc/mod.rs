// Copyright 2025 the Xilem Authors
// SPDX-License-Identifier: Apache-2.0

//! Documentation-only module for Masonry core concepts.
//!
//! This module includes a series of articles documenting internals and fundamental
//! concepts of the crate:
//!
//! - **Masonry pass system:** Deep dive into Masonry internals.
//! - **Concepts and definitions:** Glossary of concepts used in Masonry APIs and internals.

#[doc = include_str!("./pass_system.md")]
/// <style> .rustdoc-hidden { display: none; } </style>
pub mod internals_01_pass_system {}

#[doc(alias = "glossary")]
#[doc = include_str!("./masonry_concepts.md")]
/// <style> .rustdoc-hidden { display: none; } </style>
pub mod internals_02_masonry_concepts {}

// We add some aliases below so that the rest of the doc can link to these documents
// without including the chapter number.

#[doc(hidden)]
pub use self::internals_01_pass_system as pass_system;
#[doc(hidden)]
pub use self::internals_02_masonry_concepts as masonry_concepts;
