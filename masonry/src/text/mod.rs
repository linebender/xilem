// Copyright 2018 the Xilem Authors and the Druid Authors
// SPDX-License-Identifier: Apache-2.0

//! Support for text display and rendering
//!
//! There are three kinds of text commonly needed:
//!  1) Entirely display text (e.g. a button)
//!  2) Selectable text (e.g. a paragraph of content)
//!  3) Editable text (e.g. a search bar)
//!
//! All of these have the same set of global styling options, and can contain rich text

mod editor;
mod render_text;
mod styleset;

pub use editor::{ActiveText, Generation, PlainEditor, PlainEditorTxn};
pub use render_text::render_text;

/// A reference counted string slice.
///
/// This is a data-friendly way to represent strings in Masonry. Unlike `String`
/// it cannot be mutated, but unlike `String` it can be cheaply cloned.
pub type ArcStr = std::sync::Arc<str>;

#[derive(Clone, PartialEq, Default, Debug)]
pub struct BrushIndex(pub usize);

pub type StyleProperty = parley::StyleProperty<'static, BrushIndex>;

pub type StyleSet = styleset::StyleSet<BrushIndex>;
