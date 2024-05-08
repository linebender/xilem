// Copyright 2018 the Xilem Authors and the Druid Authors
// SPDX-License-Identifier: Apache-2.0

//! Rich text with style spans.

use std::ops::{Range, RangeBounds};
use std::sync::Arc;

use parley::context::RangedBuilder;
use parley::fontique::{Style, Weight};
use parley::style::FontFamily;
use vello::peniko::{Brush, Color};

use super::attribute::{Attribute, AttributeSpans, Link};
use super::font_descriptor::FontDescriptor;
use super::storage::TextStorage;
use super::util;
use crate::ArcStr;

/// Text with optional style spans.
#[derive(Clone, Debug)]
pub struct RichText {
    buffer: ArcStr,
    attrs: Arc<AttributeSpans>,
    links: Arc<[Link]>,
}

impl RichText {
    /// Create a new `RichText` object with the provided text.
    pub fn new(buffer: ArcStr) -> Self {
        RichText::new_with_attributes(buffer, Default::default())
    }

    /// Create a new `RichText`, providing explicit attributes.
    pub fn new_with_attributes(buffer: ArcStr, attributes: AttributeSpans) -> Self {
        RichText {
            buffer,
            attrs: Arc::new(attributes),
            links: Arc::new([]),
        }
    }

    /// Builder-style method for adding an [`Attribute`] to a range of text.
    ///
    /// [`Attribute`]: enum.Attribute.html
    pub fn with_attribute(mut self, range: impl RangeBounds<usize>, attr: Attribute) -> Self {
        self.add_attribute(range, attr);
        self
    }

    /// The length of the buffer, in utf8 code units.
    pub fn len(&self) -> usize {
        self.buffer.len()
    }

    /// Returns `true` if the underlying buffer is empty.
    pub fn is_empty(&self) -> bool {
        self.buffer.is_empty()
    }

    /// Add an [`Attribute`] to the provided range of text.
    ///
    /// [`Attribute`]: enum.Attribute.html
    pub fn add_attribute(&mut self, range: impl RangeBounds<usize>, attr: Attribute) {
        let range = util::resolve_range(range, self.buffer.len());
        Arc::make_mut(&mut self.attrs).add(range, attr);
    }
}

impl TextStorage for RichText {
    fn as_str(&self) -> &str {
        &self.buffer
    }
    fn add_attributes(
        &self,
        mut builder: RangedBuilder<'_, Brush, &str>,
    ) -> RangedBuilder<'_, Brush, &str> {
        for (range, attr) in self.attrs.to_piet_attrs() {
            builder.push(&attr, range);
        }
        builder
    }

    fn links(&self) -> &[Link] {
        &self.links
    }

    fn maybe_eq(&self, other: &Self) -> bool {
        Arc::ptr_eq(&self.buffer, &other.buffer)
            && Arc::ptr_eq(&self.attrs, &other.attrs)
            && Arc::ptr_eq(&self.links, &other.links)
    }
}

/// A builder for creating [`RichText`] objects.
///
/// This builder allows you to construct a [`RichText`] object by building up a sequence
/// of styled sub-strings; first you [`push`](RichTextBuilder::push) a `&str` onto the string,
/// and then you can optionally add styles to that text via the returned [`AttributesAdder`]
/// object.
///
/// # Example
/// ```
/// # use masonry::text::{Attribute, RichTextBuilder};
/// # use masonry::Color;
/// # use masonry::piet::FontWeight;
/// let mut builder = RichTextBuilder::new();
/// builder.push("Hello ");
/// builder.push("World!").weight(FontWeight::BOLD);
///
/// // Can also use write!
/// write!(builder, "Here is your number: {}", 1).underline(true).text_color(Color::RED);
///
/// let rich_text = builder.build();
/// ```
///
/// [`RichText`]: RichText
#[derive(Default)]
pub struct RichTextBuilder {
    buffer: String,
    attrs: AttributeSpans,
    links: Vec<Link>,
}

impl RichTextBuilder {
    /// Create a new `RichTextBuilder`.
    pub fn new() -> Self {
        Self::default()
    }

    /// Append a `&str` to the end of the text.
    ///
    /// This method returns a [`AttributesAdder`] that can be used to style the newly
    /// added string slice.
    pub fn push(&mut self, string: &str) -> AttributesAdder {
        let range = self.buffer.len()..(self.buffer.len() + string.len());
        self.buffer.push_str(string);
        self.add_attributes_for_range(range)
    }

    /// Glue for usage of the write! macro.
    ///
    /// This method should generally not be invoked manually, but rather through the write! macro itself.
    #[doc(hidden)]
    pub fn write_fmt(&mut self, fmt: std::fmt::Arguments<'_>) -> AttributesAdder {
        use std::fmt::Write;
        let start = self.buffer.len();
        self.buffer
            .write_fmt(fmt)
            .expect("a formatting trait implementation returned an error");
        self.add_attributes_for_range(start..self.buffer.len())
    }

    /// Get an [`AttributesAdder`] for the given range.
    ///
    /// This can be used to modify styles for a given range after it has been added.
    pub fn add_attributes_for_range(&mut self, range: impl RangeBounds<usize>) -> AttributesAdder {
        let range = util::resolve_range(range, self.buffer.len());
        AttributesAdder {
            rich_text_builder: self,
            range,
        }
    }

    /// Build the `RichText`.
    pub fn build(self) -> RichText {
        RichText {
            buffer: self.buffer.into(),
            attrs: self.attrs.into(),
            links: self.links.into(),
        }
    }
}

/// Adds Attributes to the text.
///
/// See also: [`RichTextBuilder`](RichTextBuilder)
pub struct AttributesAdder<'a> {
    rich_text_builder: &'a mut RichTextBuilder,
    range: Range<usize>,
}

impl AttributesAdder<'_> {
    /// Add the given attribute.
    pub fn add_attr(&mut self, attr: Attribute) -> &mut Self {
        self.rich_text_builder.attrs.add(self.range.clone(), attr);
        self
    }

    /// Add a font size attribute.
    pub fn size(&mut self, size: impl Into<f32>) -> &mut Self {
        self.add_attr(Attribute::size(size));
        self
    }

    /// Add a foreground color attribute.
    pub fn text_color(&mut self, color: impl Into<Color>) -> &mut Self {
        self.add_attr(Attribute::text_color(color));
        self
    }

    /// Add a font family attribute.
    pub fn font_family(&mut self, family: FontFamily<'static>) -> &mut Self {
        self.add_attr(Attribute::font_family(family));
        self
    }

    /// Add a `FontWeight` attribute.
    pub fn weight(&mut self, weight: Weight) -> &mut Self {
        self.add_attr(Attribute::weight(weight));
        self
    }

    /// Add a `FontStyle` attribute.
    pub fn style(&mut self, style: Style) -> &mut Self {
        self.add_attr(Attribute::style(style));
        self
    }

    /// Add a underline attribute.
    pub fn underline(&mut self, underline: bool) -> &mut Self {
        self.add_attr(Attribute::underline(underline));
        self
    }

    /// Add a `FontDescriptor` attribute.
    pub fn font_descriptor(&mut self, font: impl Into<FontDescriptor>) -> &mut Self {
        self.add_attr(Attribute::font_descriptor(font));
        self
    }

    //pub fn link(&mut self, command: impl Into<Command>) -> &mut Self;
}
