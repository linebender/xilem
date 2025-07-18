// Copyright 2024 the Xilem Authors
// SPDX-License-Identifier: Apache-2.0

use masonry::core::{ArcStr, StyleProperty};
use masonry::parley::style::{FontStack, FontWeight};
use masonry::properties::{DisabledTextColor, TextColor};
use masonry::widgets::{
    LineBreaking, {self},
};

use crate::core::{DynMessage, Mut, ViewMarker};
use crate::style::Style;
use crate::{MessageResult, Pod, PropertyTuple as _, TextAlign, View, ViewCtx, ViewId};

/// A non-interactive text element.
/// # Example
///
/// ```ignore
/// use xilem::palette;
/// use xilem::view::label;
/// use masonry::TextAlign;
/// use masonry::parley::fontique;
///
/// label("Text example.")
///     .text_color(palette::css::RED)
///     .text_alignment(TextAlign::Middle)
///     .text_size(24.0)
///     .weight(FontWeight::BOLD)
///     .with_font(fontique::GenericFamily::Serif)
/// ```
pub fn label(label: impl Into<ArcStr>) -> Label {
    Label {
        label: label.into(),
        text_alignment: TextAlign::default(),
        text_size: masonry::theme::TEXT_SIZE_NORMAL,
        weight: FontWeight::NORMAL,
        font: FontStack::List(std::borrow::Cow::Borrowed(&[])),
        line_break_mode: LineBreaking::Overflow,
        properties: LabelProps::default(),
    }
}

/// The [`View`] created by [`label`] from a text which `impl Into<`[`ArcStr`]`>`.
///
/// See `label` documentation for more context.
#[must_use = "View values do nothing unless provided to Xilem."]
pub struct Label {
    label: ArcStr,
    text_alignment: TextAlign,
    text_size: f32,
    weight: FontWeight,
    font: FontStack<'static>,
    line_break_mode: LineBreaking, // TODO: add more attributes of `masonry::widgets::Label`
    properties: LabelProps,
}

impl Label {
    /// Sets text alignment: `Start`, `Middle`, `End` or `Justified`.
    pub fn text_alignment(mut self, text_alignment: TextAlign) -> Self {
        self.text_alignment = text_alignment;
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

impl Style for Label {
    type Props = LabelProps;

    fn properties(&mut self) -> &mut Self::Props {
        &mut self.properties
    }
}

crate::declare_property_tuple!(
    LabelProps;
    Label;

    TextColor, 0;
    DisabledTextColor, 1;
);

impl ViewMarker for Label {}
impl<State, Action> View<State, Action, ViewCtx> for Label {
    type Element = Pod<widgets::Label>;
    type ViewState = ();

    fn build(&self, ctx: &mut ViewCtx, _: &mut State) -> (Self::Element, Self::ViewState) {
        let mut pod = ctx.create_pod(
            widgets::Label::new(self.label.clone())
                .with_text_alignment(self.text_alignment)
                .with_style(StyleProperty::FontSize(self.text_size))
                .with_style(StyleProperty::FontWeight(self.weight))
                .with_style(StyleProperty::FontStack(self.font.clone()))
                .with_line_break_mode(self.line_break_mode),
        );
        pod.properties = self.properties.build_properties();
        (pod, ())
    }

    fn rebuild(
        &self,
        prev: &Self,
        (): &mut Self::ViewState,
        _ctx: &mut ViewCtx,
        mut element: Mut<'_, Self::Element>,
        _: &mut State,
    ) {
        self.properties
            .rebuild_properties(&prev.properties, &mut element);
        if prev.label != self.label {
            widgets::Label::set_text(&mut element, self.label.clone());
        }
        if prev.text_alignment != self.text_alignment {
            widgets::Label::set_text_alignment(&mut element, self.text_alignment);
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

    fn teardown(
        &self,
        (): &mut Self::ViewState,
        _: &mut ViewCtx,
        _: Mut<'_, Self::Element>,
        _: &mut State,
    ) {
    }

    fn message(
        &self,
        (): &mut Self::ViewState,
        _id_path: &[ViewId],
        message: DynMessage,
        _app_state: &mut State,
    ) -> MessageResult<Action> {
        tracing::error!(
            "Message arrived in Label::message, but Label doesn't consume any messages, this is a bug"
        );
        MessageResult::Stale(message)
    }
}
