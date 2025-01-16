// Copyright 2024 the Xilem Authors
// SPDX-License-Identifier: Apache-2.0

use masonry::parley::style::{FontStack, FontWeight};
use masonry::text::{ArcStr, StyleProperty};
use masonry::widget;
use vello::peniko::Brush;

use crate::core::{DynMessage, Mut, ViewMarker};
use crate::{Affine, Color, MessageResult, Pod, TextAlignment, View, ViewCtx, ViewId};

use super::Transformable;

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
        transform: Affine::IDENTITY,
    }
}

/// The [`View`] created by [`label`] from a text which `impl Into<`[`ArcStr`]`>`.
///
/// See `label` documentation for more context.
#[must_use = "View values do nothing unless provided to Xilem."]
pub struct Label {
    // Public for button and variable_label as a semi-interim state.
    pub(in crate::view) label: ArcStr,
    pub(in crate::view) text_brush: Brush,
    pub(in crate::view) alignment: TextAlignment,
    pub(in crate::view) text_size: f32,
    pub(in crate::view) weight: FontWeight,
    pub(in crate::view) font: FontStack<'static>, // TODO: add more attributes of `masonry::widget::Label`
    pub(in crate::view) transform: Affine,
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
    pub fn with_font(mut self, font: impl Into<FontStack<'static>>) -> Self {
        self.font = font.into();
        self
    }
}

impl Transformable for Label {
    fn transform_mut(&mut self) -> &mut Affine {
        &mut self.transform
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
    type Element = Pod<widget::Label>;
    type ViewState = ();

    fn build(&self, ctx: &mut ViewCtx) -> (Self::Element, Self::ViewState) {
        let widget_pod = ctx.new_pod_with_transform(
            widget::Label::new(self.label.clone())
                .with_brush(self.text_brush.clone())
                .with_alignment(self.alignment)
                .with_style(StyleProperty::FontSize(self.text_size))
                .with_style(StyleProperty::FontWeight(self.weight))
                .with_style(StyleProperty::FontStack(self.font.clone())),
            self.transform,
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
        if prev.transform != self.transform {
            element.set_transform(self.transform);
        }
        if prev.label != self.label {
            widget::Label::set_text(&mut element, self.label.clone());
        }
        if prev.text_brush != self.text_brush {
            widget::Label::set_brush(&mut element, self.text_brush.clone());
        }
        if prev.alignment != self.alignment {
            widget::Label::set_alignment(&mut element, self.alignment);
        }
        if prev.text_size != self.text_size {
            widget::Label::insert_style(&mut element, StyleProperty::FontSize(self.text_size));
        }
        if prev.weight != self.weight {
            widget::Label::insert_style(&mut element, StyleProperty::FontWeight(self.weight));
        }
        if prev.font != self.font {
            widget::Label::insert_style(&mut element, StyleProperty::FontStack(self.font.clone()));
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
