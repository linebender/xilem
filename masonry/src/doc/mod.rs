// Copyright 2024 the Xilem Authors
// SPDX-License-Identifier: Apache-2.0

//! Documentation-only module
//!
//! This module includes a series of articles documenting the crate:
//!
//! - **Building a "To-Do List" app:** Tutorial to get started with Masonry.
//! - **Creating a new widget:** Introduces the `Widget` trait.
//! - **Creating a container widget:** Expands on the `Widget` trait for container widgets.
//! - **Testing widgets in Masonry:** Describes how to test your Masonry widgets in CI.
//! - **Masonry pass system:** Deep dive into Masonry internals.
//! - **Concepts and definitions:** Glossary of concepts used in Masonry APIs and internals.

// These docs all use the .rustdoc-hidden trick described in
// https://linebender.org/blog/doc-include/

/// Contains the items implemented in "Creating a new widget" and other tutorials.
pub mod color_rectangle;

// TODO - Add vertical_stack module.

#[doc = include_str!("./creating_app.md")]
/// <style> .rustdoc-hidden { display: none; } </style>
pub mod doc_01_creating_app {}

#[doc = include_str!("./implementing_widget.md")]
/// <style> .rustdoc-hidden { display: none; } </style>
pub mod doc_02_implementing_widget {}

#[doc = include_str!("./implementing_container_widget.md")]
/// <style> .rustdoc-hidden { display: none; } </style>
pub mod doc_03_implementing_container_widget {}

#[doc = include_str!("./testing_widget.md")]
/// <style> .rustdoc-hidden { display: none; } </style>
pub mod doc_04_testing_widget {}

#[doc = include_str!("./widget_properties.md")]
/// <style> .rustdoc-hidden { display: none; } </style>
pub mod doc_05_widget_properties {}

pub use masonry_core::doc::internals_01_pass_system;
pub use masonry_core::doc::internals_02_masonry_concepts;
