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

#![warn(missing_docs)]
mod render_text;

use parley::GenericFamily;
pub use render_text::render_text;

/// A reference counted string slice.
///
/// This is a data-friendly way to represent strings in Masonry. Unlike `String`
/// it cannot be mutated, but unlike `String` it can be cheaply cloned.
pub type ArcStr = std::sync::Arc<str>;

/// The Parley [`parley::Brush`] used within Masonry.
///
/// This enables updating of brush details without performing relayouts;
/// the inner values are indexes into the `brushes` argument to [`render_text`].
#[derive(Clone, PartialEq, Default, Debug)]
pub struct BrushIndex(pub usize);

/// A style property specialised for use within Masonry.
pub type StyleProperty = parley::StyleProperty<'static, BrushIndex>;

/// A set of styles specialised for use within Masonry.
pub type StyleSet = parley::StyleSet<BrushIndex>;

/// Applies the default text styles for Masonry into `styles`.
pub(crate) fn default_styles(styles: &mut StyleSet) {
    styles.insert(StyleProperty::LineHeight(1.2));
    styles.insert(GenericFamily::SystemUi.into());
}
