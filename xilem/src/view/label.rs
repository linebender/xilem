// Copyright 2024 the Xilem Authors
// SPDX-License-Identifier: Apache-2.0

use masonry::parley::style::FontStack;
use masonry::parley::style::FontWeight;
use masonry::core::ArcStr;
use masonry::core::StyleProperty;
use masonry::widgets::LineBreaking;
use masonry::widgets::{self};
use vello::peniko::Brush;

use crate::core::DynMessage;
use crate::core::Mut;
use crate::core::ViewMarker;
use crate::Color;
use crate::MessageResult;
use crate::Pod;
use crate::TextAlignment;
use crate::View;
use crate::ViewCtx;
use crate::ViewId;

/// A non-interactive text element.
/// # Example
///
/// ```ignore
/// use xilem::palette;
/// use xilem::view::label;
/// use masonry::TextAlignment;
/// use masonry::parley::fontique;
///
/// label("Text example.")
///     .brush(palette::css::RED)
///     .alignment(TextAlignment::Middle)
///     .text_size(24.0)
///     .weight(FontWeight::BOLD)
///     .with_font(fontique::GenericFamily::Serif)
/// ```
pub fn label(label: impl Into<ArcStr>) -> Label {
    Label {
        label: label.into(),
        text_brush: Color::WHITE.into(),
        alignment: TextAlignment::default(),
        text_size: masonry::theme::TEXT_SIZE_NORMAL,
        weight: FontWeight::NORMAL,
        font: FontStack::List(std::borrow::Cow::Borrowed(&[])),
        line_break_mode: LineBreaking::Overflow,
    }
}

/// The [`View`] created by [`label`] from a text which `impl Into<`[`ArcStr`]`>`.
///
/// See `label` documentation for more context.
#[must_use = "View values do nothing unless provided to Xilem."]
pub struct Label {
    label: ArcStr,
    text_brush: Brush,
    alignment: TextAlignment,
    text_size: f32,
    weight: FontWeight,
    font: FontStack<'static>,
    line_break_mode: LineBreaking, // TODO: add more attributes of `masonry::widgets::Label`
}

impl Label {
    /// In most cases brush sets text color, but gradients and images are also supported.
    #[doc(alias = "color")]
    pub fn brush(mut self, brush: impl Into<Brush>) -> Self {
        self.text_brush = brush.into();
        self
    }

    /// Sets text alignment: `Start`, `Middle`, `End` or `Justified`.
    pub fn alignment(mut self, alignment: TextAlignment) -> Self {
        self.alignment = alignment;
        self
    }

    /// Sets text size.
    #[doc(alias = "font_size")]
    pub fn text_size(mut self, text_size: f32) -> Self {
        self.text_size = text_size;
        self
    }

    /// Sets font weight.
    pub fn weight(mut self, weight: FontWeight) -> Self {
        self.weight = weight;
        self
    }

    /// Set the [font stack](FontStack) this label will use.
    ///
    /// A font stack allows for providing fallbacks. If there is no matching font
    /// for a character, a system font will be used (if the system fonts are enabled).
    pub fn font(mut self, font: impl Into<FontStack<'static>>) -> Self {
        self.font = font.into();
        self
    }

    /// Set how line breaks will be handled by this label (i.e. if there is insufficient horizontal space).
    pub fn line_break_mode(mut self, line_break_mode: LineBreaking) -> Self {
        self.line_break_mode = line_break_mode;
        self
    }
}

impl<T> From<T> for Label
where
    T: Into<ArcStr>,
{
    fn from(text: T) -> Self {
        label(text)
    }
}

impl ViewMarker for Label {}
impl<State, Action> View<State, Action, ViewCtx> for Label {
    type Element = Pod<widgets::Label>;
    type ViewState = ();

    fn build(&self, ctx: &mut ViewCtx) -> (Self::Element, Self::ViewState) {
        let widget_pod = ctx.new_pod(
            widgets::Label::new(self.label.clone())
                .with_brush(self.text_brush.clone())
                .with_alignment(self.alignment)
                .with_style(StyleProperty::FontSize(self.text_size))
                .with_style(StyleProperty::FontWeight(self.weight))
                .with_style(StyleProperty::FontStack(self.font.clone()))
                .with_line_break_mode(self.line_break_mode),
        );
        (widget_pod, ())
    }

    fn rebuild(
        &self,
        prev: &Self,
        (): &mut Self::ViewState,
        _ctx: &mut ViewCtx,
        mut element: Mut<Self::Element>,
    ) {
        if prev.label != self.label {
            widgets::Label::set_text(&mut element, self.label.clone());
        }
        if prev.text_brush != self.text_brush {
            widgets::Label::set_brush(&mut element, self.text_brush.clone());
        }
        if prev.alignment != self.alignment {
            widgets::Label::set_alignment(&mut element, self.alignment);
        }
        if prev.text_size != self.text_size {
            widgets::Label::insert_style(&mut element, StyleProperty::FontSize(self.text_size));
        }
        if prev.weight != self.weight {
            widgets::Label::insert_style(&mut element, StyleProperty::FontWeight(self.weight));
        }
        if prev.font != self.font {
            widgets::Label::insert_style(&mut element, StyleProperty::FontStack(self.font.clone()));
        }
        if prev.line_break_mode != self.line_break_mode {
            widgets::Label::set_line_break_mode(&mut element, self.line_break_mode);
        }
    }

    fn teardown(&self, (): &mut Self::ViewState, _: &mut ViewCtx, _: Mut<Self::Element>) {}

    fn message(
        &self,
        (): &mut Self::ViewState,
        _id_path: &[ViewId],
        message: DynMessage,
        _app_state: &mut State,
    ) -> MessageResult<Action> {
        tracing::error!("Message arrived in Label::message, but Label doesn't consume any messages, this is a bug");
        MessageResult::Stale(message)
    }
}
