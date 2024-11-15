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

mod backspace;
mod edit;
mod editor;
mod render_text;
mod selection;

use std::{collections::HashMap, mem::Discriminant};

pub use backspace::offset_for_delete_backwards;
pub use edit::TextEditor;
pub use render_text::render_text;
pub use selection::{len_utf8_from_first_byte, Selectable, StringCursor, TextWithSelection};

/// A reference counted string slice.
///
/// This is a data-friendly way to represent strings in Masonry. Unlike `String`
/// it cannot be mutated, but unlike `String` it can be cheaply cloned.
pub type ArcStr = std::sync::Arc<str>;

#[derive(Clone, PartialEq, Default, Debug)]
pub struct BrushIndex(pub usize);

pub type StyleProperty = parley::StyleProperty<'static, BrushIndex>;

/// A set of Parley styles.
pub struct StyleSet(HashMap<Discriminant<StyleProperty>, StyleProperty>);

impl StyleSet {
    pub fn new(font_size: f32) -> Self {
        let mut this = Self(Default::default());
        this.insert(StyleProperty::FontSize(font_size));
        this
    }

    pub fn insert(&mut self, style: StyleProperty) -> Option<StyleProperty> {
        let discriminant = std::mem::discriminant(&style);
        self.0.insert(discriminant, style)
    }

    pub fn retain(&mut self, mut f: impl FnMut(&StyleProperty) -> bool) {
        self.0.retain(|_, v| f(v));
    }

    pub fn remove(&mut self, property: Discriminant<StyleProperty>) -> Option<StyleProperty> {
        self.0.remove(&property)
    }

    pub fn inner(&self) -> &HashMap<Discriminant<StyleProperty>, StyleProperty> {
        &self.0
    }
}
