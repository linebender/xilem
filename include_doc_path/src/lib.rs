// Copyright 2026 the Xilem Authors
// SPDX-License-Identifier: Apache-2.0

//! This crate provides the [`include_doc_path!`] macro which takes a path to a local resource and always returns a valid URL to it.
//!
//! - If the path points to an existing local file, it returns that path.
//! - Otherwise, it returns a `raw.githubusercontent.com` URL to the file.
//!
//! This is useful for pointing to files such as screenshots and videos which you might want to exclude from a crate's package, but still include in the doc.
//!
//! If the fallback is taken, the generated URL is
//! `https://raw.githubusercontent.com/{owner}/{name}/{tag_prefix}{crate_version}/{crate_name}/{relative_path}`. This has a few baked-in assumptions:
//!
//! - Your Git repository forge is GitHub.
//! - Your crate is in a `crate_name` folder at the root of the repository.
//! - The file you're linking to is in that folder.
//! - Your repository gets a Git tag with the crate version number every time the crate releases.
//!
//! The `tag_prefix` string is currently hardcoded to `v` (e.g. `v1.2.3`), but might become configurable in future versions.
//!
//! Otherwise, those assumptions are fairly opinionated, as this crate is meant primarily to be used by Linebender projects.
//!
//! # Error checking
//!
//! The URL fallback is needed when publishing to `docs.rs`, but in cases like CI you might instead want to emit a compile error when the path isn't found.
//!
//! To do so, set the `CHECK_DOC_PATHS` env variable to `true` when running the macro.
//!
//! # Cargo flags
//!
//! The macro requires the following env flags to be set:
//!
//! - `CARGO_MANIFEST_DIR`
//! - `CARGO_PKG_REPOSITORY`
//! - `CARGO_PKG_VERSION`
//! - `CARGO_PKG_NAME`
//!
//! If they aren't set (e.g. because you're using a different build system), the macro will just return the input string without modification.

use proc_macro::TokenStream;
use syn::{LitStr, parse_macro_input};

mod logic;

/// Takes a path to a local resource and always returns a valid URL to it.
///
/// See root documentation for details.
///
/// ```rust,ignore
/// #[doc = concat!(
///     "![Button with text label](",
///     include_doc_path!("screenshots/button_hello.png"),
///     ")",
/// )]
/// ```
#[proc_macro]
pub fn include_doc_path(input: TokenStream) -> TokenStream {
    let lit = parse_macro_input!(input as LitStr);
    let relative_path = lit.value();

    logic::include_doc_path_impl(relative_path, lit.span(), false, false).into()
}

/// Helper macro which always returns the same string as if the path was found.
///
/// Useful for debugging.
#[proc_macro]
pub fn helper_include_local_path(input: TokenStream) -> TokenStream {
    let lit = parse_macro_input!(input as LitStr);
    let relative_path = lit.value();

    logic::include_doc_path_impl(relative_path, lit.span(), true, false).into()
}

/// Helper macro which always returns the same string as if the path was not found.
///
/// Useful for debugging.
#[proc_macro]
pub fn helper_include_github_url(input: TokenStream) -> TokenStream {
    let lit = parse_macro_input!(input as LitStr);
    let relative_path = lit.value();

    logic::include_doc_path_impl(relative_path, lit.span(), false, true).into()
}
