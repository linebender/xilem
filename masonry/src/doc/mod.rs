// Copyright 2024 the Xilem Authors
// SPDX-License-Identifier: Apache-2.0

//! Documentation-only module
//!
//! This module includes a series of articles documenting the crate:
//!
//! - **Building a "To-Do List" app:** Tutorial to get started with Masonry.
//! - **Creating a new Widget:** Introduces the Widget trait.
//! - **Creating a container Widget:** Expands on the Widget trait for container Widgets.
//! - **Testing widgets in Masonry:** TODO.
//! - **Masonry pass system:** Deep dive into Masonry internals.
//! - **Concepts and definitions:** Glossary of concepts used in Masonry APIs and internals.

// TODO: Remove this once the issues within masonry are fixed. Tracked in https://github.com/linebender/xilem/issues/449
#![warn(rustdoc::broken_intra_doc_links)]

// These docs all use the .rustdoc-hidden trick described in
// https://linebender.org/blog/doc-include/

#[doc = include_str!("./01_creating_app.md")]
/// <style> .rustdoc-hidden { display: none; } </style>
pub mod doc_01_creating_app {}

#[doc = include_str!("./02_implementing_widget.md")]
/// <style> .rustdoc-hidden { display: none; } </style>
pub mod doc_02_implementing_widget {}

#[doc = include_str!("./03_implementing_container_widget.md")]
/// <style> .rustdoc-hidden { display: none; } </style>
pub mod doc_03_implementing_container_widget {}

#[doc = include_str!("./04_testing_widget.md")]
/// <style> .rustdoc-hidden { display: none; } </style>
pub mod doc_04_testing_widget {}

#[doc = include_str!("./05_pass_system.md")]
/// <style> .rustdoc-hidden { display: none; } </style>
pub mod doc_05_pass_system {}

#[doc = include_str!("./06_masonry_concepts.md")]
/// <style> .rustdoc-hidden { display: none; } </style>
pub mod doc_06_masonry_concepts {}
