// Copyright 2025 the Xilem Authors
// SPDX-License-Identifier: Apache-2.0

// TODO - Renumber docs.

//! Documentation-only module for Masonry core concepts.
//!
//! This module includes a series of articles documenting internals and fundamental
//! concepts of the crate:
//!
//! - **Masonry pass system:** Deep dive into Masonry internals.
//! - **Concepts and definitions:** Glossary of concepts used in Masonry APIs and internals.

#[doc = include_str!("./05_pass_system.md")]
/// <style> .rustdoc-hidden { display: none; } </style>
pub mod doc_05_pass_system {}

#[doc(alias = "glossary")]
#[doc = include_str!("./06_masonry_concepts.md")]
/// <style> .rustdoc-hidden { display: none; } </style>
pub mod doc_06_masonry_concepts {}
