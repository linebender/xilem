// Copyright 2018 the Xilem Authors and the Druid Authors
// SPDX-License-Identifier: Apache-2.0

//! Storing text.

use std::{ops::Deref, sync::Arc};

use parley::context::RangedBuilder;

use crate::ArcStr;

use super::layout::TextBrush;

#[derive(Copy, Clone)]
// TODO: Implement links
pub struct Link;

/// Text which can be displayed.
pub trait TextStorage: 'static {
    fn as_str(&self) -> &str;
    /// If this `TextStorage` object manages style spans, it should implement
    /// this method and update the provided builder with its spans, as required.
    ///
    /// This takes `&self`, as we needed to call `Self::as_str` to get the value stored in
    /// the `RangedBuilder`
    #[allow(unused_variables)]
    fn add_attributes<'b>(
        &self,
        builder: RangedBuilder<'b, TextBrush, &'b str>,
    ) -> RangedBuilder<'b, TextBrush, &'b str> {
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

impl TextStorage for &'static str {
    fn as_str(&self) -> &str {
        self
    }
    fn maybe_eq(&self, other: &Self) -> bool {
        self == other
    }
}

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
