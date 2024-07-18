// Copyright 2018 the Xilem Authors and the Druid Authors
// SPDX-License-Identifier: Apache-2.0

//! Storing text.

use std::{ops::Deref, sync::Arc};

use parley::context::RangedBuilder;
use vello::peniko::Brush;

use super::attribute::Link;

/// A type that represents text that can be displayed.
pub trait TextStorage: Clone {
    fn as_str(&self) -> &str;
    /// If this TextStorage object manages style spans, it should implement
    /// this method and update the provided builder with its spans, as required.
    #[allow(unused_variables)]
    fn add_attributes(&self, builder: RangedBuilder<Brush, &str>) -> RangedBuilder<Brush, &str> {
        builder
    }

    /// Any additional [`Link`] attributes on this text.
    ///
    /// If this `TextStorage` object manages link attributes, it should implement this
    /// method and return any attached [`Link`]s.
    ///
    /// Unlike other attributes, links are managed in Masonry, not in [`piet`]; as such they
    /// require a separate API.
    ///
    /// [`Link`]: super::attribute::Link
    /// [`piet`]: https://docs.rs/piet
    fn links(&self) -> &[Link] {
        &[]
    }

    /// Determines quickly whether two text objects have the same content.
    ///
    /// To allow for faster checks, this method is allowed to return false negatives.
    fn maybe_eq(&self, other: &Self) -> bool;
}

/// A reference counted string slice.
///
/// This is a data-friendly way to represent strings in Masonry. Unlike `String`
/// it cannot be mutated, but unlike `String` it can be cheaply cloned.
pub type ArcStr = Arc<str>;

impl TextStorage for ArcStr {
    fn as_str(&self) -> &str {
        self.deref()
    }
    fn maybe_eq(&self, other: &Self) -> bool {
        self == other
    }
}

impl TextStorage for String {
    fn as_str(&self) -> &str {
        self.deref()
    }
    fn maybe_eq(&self, other: &Self) -> bool {
        self == other
    }
}

impl TextStorage for Arc<String> {
    fn as_str(&self) -> &str {
        self.deref()
    }
    fn maybe_eq(&self, other: &Self) -> bool {
        self == other
    }
}
