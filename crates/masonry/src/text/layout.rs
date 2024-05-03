// Copyright 2018 the Xilem Authors and the Druid Authors
// SPDX-License-Identifier: Apache-2.0

//! A type for laying out, drawing, and interacting with text.

use std::rc::Rc;

use kurbo::{Line, Point, Rect, Size};
use parley::layout::{Alignment, Cursor};
use parley::style::{FontFamily, GenericFamily};
use parley::{Layout, LayoutContext};
use vello::peniko::{Brush, Color};

use crate::PaintCtx;

use super::attribute::Link;
use super::font_descriptor::FontDescriptor;
use super::storage::TextStorage;

/// A component for displaying text on screen.
///
/// This is a type intended to be used by other widgets that display text.
/// It allows for the text itself as well as font and other styling information
/// to be set and modified. It wraps an inner layout object, and handles
/// invalidating and rebuilding it as required.
///
/// This object is not valid until the [`rebuild_if_needed`] method has been
/// called. You should generally do this in your widget's [`layout`] method.
/// Additionally, you should call [`needs_rebuild_after_update`]
/// as part of your widget's [`update`] method; if this returns `true`, you will need
/// to call [`rebuild_if_needed`] again, generally by scheduling another [`layout`]
/// pass.
///
/// [`layout`]: trait.Widget.html#tymethod.layout
/// [`update`]: trait.Widget.html#tymethod.update
/// [`needs_rebuild_after_update`]: #method.needs_rebuild_after_update
/// [`rebuild_if_needed`]: #method.rebuild_if_needed
#[derive(Clone)]
pub struct TextLayout<T> {
    // TODO - remove Option
    text: Option<T>,
    font: FontDescriptor,
    // when set, this will be used to override the size in he font descriptor.
    // This provides an easy way to change only the font size, while still
    // using a `FontDescriptor` in the `Env`.
    text_size_override: Option<f32>,
    text_color: Color,
    layout: Option<Layout<Brush>>,
    wrap_width: f64,
    alignment: Alignment,
    links: Rc<[(Rect, usize)]>,
}

/// Metrics describing the layout text.
#[derive(Debug, Clone, Copy, Default)]
pub struct LayoutMetrics {
    /// The nominal size of the layout.
    pub size: Size,
    /// The distance from the nominal top of the layout to the first baseline.
    pub first_baseline: f32,
    /// The width of the layout, inclusive of trailing whitespace.
    pub trailing_whitespace_width: f32,
    //TODO: add inking_rect
}

impl<T> TextLayout<T> {
    /// Create a new `TextLayout` object.
    ///
    /// You must set the text ([`set_text`]) before using this object.
    ///
    /// [`set_text`]: #method.set_text
    pub fn new() -> Self {
        TextLayout {
            text: None,
            font: FontDescriptor::new(FontFamily::Generic(GenericFamily::SystemUi)),
            text_color: crate::theme::TEXT_COLOR,
            text_size_override: None,
            layout: None,
            wrap_width: f64::INFINITY,
            alignment: Default::default(),
            links: Rc::new([]),
        }
    }

    /// Set the default text color for this layout.
    pub fn set_text_color(&mut self, color: Color) {
        if color != self.text_color {
            self.text_color = color;
            self.layout = None;
        }
    }

    /// Set the default font.
    ///
    /// The argument is a [`FontDescriptor`].
    ///
    /// [`FontDescriptor`]: struct.FontDescriptor.html
    pub fn set_font(&mut self, font: FontDescriptor) {
        if font != self.font {
            self.font = font;
            self.layout = None;
            self.text_size_override = None;
        }
    }

    /// Set the font size.
    ///
    /// This overrides the size in the [`FontDescriptor`] provided to [`set_font`].
    ///
    /// [`set_font`]: #method.set_font.html
    /// [`FontDescriptor`]: struct.FontDescriptor.html
    pub fn set_text_size(&mut self, size: f32) {
        if Some(&size) != self.text_size_override.as_ref() {
            self.text_size_override = Some(size);
            self.layout = None;
        }
    }

    /// Set the width at which to wrap words.
    ///
    /// You may pass `f64::INFINITY` to disable word wrapping
    /// (the default behaviour).
    pub fn set_wrap_width(&mut self, width: f64) {
        let width = width.max(0.0);
        // 1e-4 is an arbitrary small-enough value that we don't care to rewrap
        if (width - self.wrap_width).abs() > 1e-4 {
            self.wrap_width = width;
            self.layout = None;
        }
    }

    /// Set the [`TextAlignment`] for this layout.
    ///
    /// [`TextAlignment`]: enum.TextAlignment.html
    pub fn set_text_alignment(&mut self, alignment: Alignment) {
        if self.alignment != alignment {
            self.alignment = alignment;
            self.layout = None;
        }
    }

    // /// Returns `true` if this layout's text appears to be right-to-left.
    // ///
    // /// See [`piet::util::first_strong_rtl`] for more information.
    // ///
    // /// [`piet::util::first_strong_rtl`]: crate::piet::util::first_strong_rtl
    // pub fn text_is_rtl(&self) -> bool {
    //     self.text_is_rtl
    // }
}

impl<T: TextStorage> TextLayout<T> {
    /// Create a new `TextLayout` with the provided text.
    ///
    /// This is useful when the text is not tied to application data.
    pub fn from_text(text: impl Into<T>) -> Self {
        let mut this = TextLayout::new();
        this.set_text(text.into());
        this
    }

    /// Returns `true` if this layout needs to be rebuilt.
    ///
    /// This happens (for instance) after style attributes are modified.
    ///
    /// This does not account for things like the text changing, handling that
    /// is the responsibility of the user.
    pub fn needs_rebuild(&self) -> bool {
        self.layout.is_none()
    }

    /// Set the text to display.
    pub fn set_text(&mut self, text: T) {
        if self.text.is_none() || !self.text.as_ref().unwrap().maybe_eq(&text) {
            self.text = Some(text);
            self.layout = None;
        }
    }

    /// Returns the [`TextStorage`] backing this layout, if it exists.
    pub fn text(&self) -> Option<&T> {
        self.text.as_ref()
    }

    /// Returns the length of the [`TextStorage`] backing this layout, if it exists.
    pub fn text_len(&self) -> usize {
        if let Some(text) = &self.text {
            text.as_str().len()
        } else {
            0
        }
    }

    /// Returns the inner Piet [`TextLayout`] type.
    ///
    /// [`TextLayout`]: ./piet/trait.TextLayout.html
    pub fn layout(&self) -> Option<&Layout<Brush>> {
        self.layout.as_ref()
    }

    /// The size of the laid-out text.
    ///
    /// This is not meaningful until [`rebuild_if_needed`] has been called.
    ///
    /// [`rebuild_if_needed`]: #method.rebuild_if_needed
    pub fn size(&self) -> Size {
        self.layout
            .as_ref()
            .map(|layout| Size::new(layout.width().into(), layout.height().into()))
            .unwrap_or_default()
    }

    /// Return the text's [`LayoutMetrics`].
    ///
    /// This is not meaningful until [`rebuild_if_needed`] has been called.
    ///
    /// [`rebuild_if_needed`]: #method.rebuild_if_needed
    /// [`LayoutMetrics`]: struct.LayoutMetrics.html
    pub fn layout_metrics(&self) -> LayoutMetrics {
        debug_assert!(
            self.layout.is_some(),
            "TextLayout::layout_metrics called without rebuilding layout object. Text was '{}'",
            self.text().as_ref().map(|s| s.as_str()).unwrap_or_default()
        );

        if let Some(layout) = self.layout.as_ref() {
            let first_baseline = layout.get(0).unwrap().metrics().baseline;
            let size = Size::new(layout.width().into(), layout.height().into());
            LayoutMetrics {
                size,
                first_baseline,
                trailing_whitespace_width: layout.width(),
            }
        } else {
            LayoutMetrics::default()
        }
    }

    /// For a given `Point` (relative to this object's origin), returns index
    /// into the underlying text of the nearest grapheme boundary.
    pub fn text_position_for_point(&self, point: Point) -> usize {
        self.layout
            .as_ref()
            .map(|layout| Cursor::from_point(layout, point.x as f32, point.y as f32).insert_point)
            .unwrap_or_default()
    }

    /// Given the utf-8 position of a character boundary in the underlying text,
    /// return the `Point` (relative to this object's origin) representing the
    /// boundary of the containing grapheme.
    ///
    /// # Panics
    ///
    /// Panics if `text_pos` is not a character boundary.
    pub fn point_for_text_position(&self, text_pos: usize) -> Point {
        self.layout
            .as_ref()
            .map(|layout| {
                let from_position = Cursor::from_position(layout, text_pos, /* TODO */ false);

                Point::new(
                    from_position.advance as f64,
                    (from_position.baseline + from_position.offset) as f64,
                )
            })
            .unwrap_or_default()
    }

    // /// Given a utf-8 range in the underlying text, return a `Vec` of `Rect`s
    // /// representing the nominal bounding boxes of the text in that range.
    // ///
    // /// # Panics
    // ///
    // /// Panics if the range start or end is not a character boundary.
    // pub fn rects_for_range(&self, range: Range<usize>) -> Vec<Rect> {
    //     self.layout
    //         .as_ref()
    //         .map(|layout| layout.rects_for_range(range))
    //         .unwrap_or_default()
    // }

    /// Given the utf-8 position of a character boundary in the underlying text,
    /// return a `Line` suitable for drawing a vertical cursor at that boundary.
    pub fn cursor_line_for_text_position(&self, text_pos: usize) -> Line {
        self.layout
            .as_ref()
            .map(|layout| {
                let from_position = Cursor::from_position(layout, text_pos, /* TODO */ false);

                let line_metrics = from_position.path.line(layout).unwrap().metrics();

                let p1 = (from_position.advance as f64, line_metrics.baseline as f64);
                let p2 = (
                    from_position.advance as f64,
                    (line_metrics.baseline + line_metrics.size()) as f64,
                );
                Line::new(p1, p2)
            })
            .unwrap_or_else(|| Line::new(Point::ZERO, Point::ZERO))
    }

    /// Returns the [`Link`] at the provided point (relative to the layout's origin) if one exists.
    ///
    /// This can be used both for hit-testing (deciding whether to change the mouse cursor,
    /// or performing some other action when hovering) as well as for retrieving a [`Link`]
    /// on click.
    ///
    /// [`Link`]: super::attribute::Link
    pub fn link_for_pos(&self, pos: Point) -> Option<&Link> {
        let (_, i) = self
            .links
            .iter()
            .rfind(|(hit_box, _)| hit_box.contains(pos))?;

        let text = self.text()?;
        text.links().get(*i)
    }

    /// Rebuild the inner layout as needed.
    ///
    /// This `TextLayout` object manages a lower-level layout object that may
    /// need to be rebuilt in response to changes to the text or attributes
    /// like the font.
    ///
    /// This method should be called whenever any of these things may have changed.
    /// A simple way to ensure this is correct is to always call this method
    /// as part of your widget's [`layout`] method.
    ///
    /// [`layout`]: trait.Widget.html#method.layout
    pub fn rebuild_if_needed(&mut self, factory: &mut LayoutContext<Brush>) {
        if let Some(text) = &self.text {
            if self.layout.is_none() {
                let font = self.font.clone();
                let color = self.text_color;
                let size_override = self.text_size_override;

                let descriptor = if let Some(size) = size_override {
                    font.with_size(size)
                } else {
                    font
                };

                let builder = factory.ranged_builder(fcx, text, 1.0);
                builder
                    .push_default(StyleProperty)
                    .new_text_layout(text.clone())
                    .max_width(self.wrap_width)
                    .alignment(self.alignment)
                    .font(descriptor.family.clone(), descriptor.size)
                    .default_attribute(descriptor.weight)
                    .default_attribute(descriptor.style);
                // .default_attribute(TextAttribute::TextColor(color));
                let layout = text.add_attributes(builder).build().unwrap();

                self.links = text
                    .links()
                    .iter()
                    .enumerate()
                    .flat_map(|(i, link)| {
                        layout
                            .rects_for_range(link.range())
                            .into_iter()
                            .map(move |rect| (rect, i))
                    })
                    .collect();

                self.layout = Some(layout);
            }
        }
    }

    ///  Draw the layout at the provided `Point`.
    ///
    ///  The origin of the layout is the top-left corner.
    ///
    ///  You must call [`rebuild_if_needed`] at some point before you first
    ///  call this method.
    ///
    ///  [`rebuild_if_needed`]: #method.rebuild_if_needed
    pub fn draw(&self, ctx: &mut PaintCtx, point: impl Into<Point>) {
        debug_assert!(
            self.layout.is_some(),
            "TextLayout::draw called without rebuilding layout object. Text was '{}'",
            self.text
                .as_ref()
                .map(|t| t.as_str())
                .unwrap_or("layout is missing text")
        );
        if let Some(layout) = self.layout.as_ref() {
            ctx.draw_text(layout, point);
        }
    }
}

impl<T> std::fmt::Debug for TextLayout<T> {
    fn fmt(&self, f: &mut std::fmt::Formatter) -> std::fmt::Result {
        f.debug_struct("TextLayout")
            .field("font", &self.font)
            .field("text_size_override", &self.text_size_override)
            .field("text_color", &self.text_color)
            .field(
                "layout",
                if self.layout.is_some() {
                    &"Some"
                } else {
                    &"None"
                },
            )
            .finish()
    }
}

impl<T: TextStorage> Default for TextLayout<T> {
    fn default() -> Self {
        Self::new()
    }
}
