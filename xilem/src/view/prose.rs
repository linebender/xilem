// Copyright 2024 the Xilem Authors
// SPDX-License-Identifier: Apache-2.0

use std::marker::PhantomData;

use masonry::core::{ArcStr, NewWidget, Properties, StyleProperty};
use masonry::parley::FontWeight;
use masonry::properties::{ContentColor, DisabledContentColor, LineBreaking};
use masonry::widgets;

use crate::core::{Arg, MessageContext, Mut, View, ViewArgument, ViewMarker};
use crate::{Color, MessageResult, Pod, TextAlign, ViewCtx};

/// A view which displays selectable text.
pub fn prose<State, Action>(content: impl Into<ArcStr>) -> Prose<State, Action> {
    Prose {
        content: content.into(),
        text_color: None,
        disabled_text_color: None,
        text_alignment: TextAlign::default(),
        text_size: masonry::theme::TEXT_SIZE_NORMAL,
        line_break_mode: LineBreaking::WordWrap,
        weight: FontWeight::NORMAL,
        phantom: PhantomData,
    }
}

/// A version of [`prose`] suitable for including in the same line
/// as other content.
///
/// Note that setting [`text_alignment`](Prose::text_alignment) on the result
/// will be meaningless.
#[doc(alias = "span")]
pub fn inline_prose<State, Action>(content: impl Into<ArcStr>) -> Prose<State, Action> {
    prose(content).line_break_mode(LineBreaking::Overflow)
}

/// The [`View`] created by [`prose`] or [`inline_prose`].
#[must_use = "View values do nothing unless provided to Xilem."]
pub struct Prose<State, Action> {
    content: ArcStr,

    text_color: Option<Color>,
    disabled_text_color: Option<Color>,
    text_alignment: TextAlign,
    text_size: f32,
    line_break_mode: LineBreaking,
    weight: FontWeight,
    phantom: PhantomData<fn(State) -> Action>,
    // TODO: disabled: bool,
    // TODO: add more attributes of `masonry::widgets::Prose`
}

impl<State, Action> Prose<State, Action> {
    /// Set the text's color.
    ///
    /// This overwrites the default `ContentColor` property for the inner `TextArea` widget.
    pub fn text_color(mut self, color: Color) -> Self {
        self.text_color = Some(color);
        self
    }

    /// Set the text's color when the text input is disabled.
    ///
    /// This overwrites the default `DisabledContentColor` property for the inner `TextArea` widget.
    pub fn disabled_text_color(mut self, color: Color) -> Self {
        self.disabled_text_color = Some(color);
        self
    }

    /// Set the [text alignment](https://en.wikipedia.org/wiki/Typographic_alignment) of the text.
    pub fn text_alignment(mut self, text_alignment: TextAlign) -> Self {
        self.text_alignment = text_alignment;
        self
    }

    /// Set the font size of the text.
    #[doc(alias = "font_size")]
    pub fn text_size(mut self, text_size: f32) -> Self {
        self.text_size = text_size;
        self
    }

    /// Set how the text is broken when the content is too wide for its container.
    pub fn line_break_mode(mut self, line_break_mode: LineBreaking) -> Self {
        self.line_break_mode = line_break_mode;
        self
    }

    /// Sets font weight.
    pub fn weight(mut self, weight: FontWeight) -> Self {
        self.weight = weight;
        self
    }
}

fn line_break_clips(linebreaking: LineBreaking) -> bool {
    matches!(linebreaking, LineBreaking::Clip | LineBreaking::WordWrap)
}

impl<State, Action> ViewMarker for Prose<State, Action> {}
impl<State: ViewArgument, Action: 'static> View<State, Action, ViewCtx> for Prose<State, Action> {
    type Element = Pod<widgets::Prose>;
    type ViewState = ();

    fn build(&self, ctx: &mut ViewCtx, _: Arg<'_, State>) -> (Self::Element, Self::ViewState) {
        let text_area = widgets::TextArea::new_immutable(&self.content)
            .with_text_alignment(self.text_alignment)
            .with_style(StyleProperty::FontSize(self.text_size))
            .with_style(StyleProperty::FontWeight(self.weight))
            .with_word_wrap(self.line_break_mode == LineBreaking::WordWrap);

        // TODO - Replace this with properties on the Prose view
        // once we implement property inheritance or something like it.
        let mut props = Properties::new();
        if let Some(color) = self.text_color {
            props.insert(ContentColor { color });
        }
        if let Some(color) = self.disabled_text_color {
            props.insert(DisabledContentColor(ContentColor { color }));
        }
        let text_area = NewWidget::new_with_props(text_area, props);

        let pod = ctx.create_pod(
            widgets::Prose::from_text_area(text_area)
                .with_clip(line_break_clips(self.line_break_mode)),
        );
        (pod, ())
    }

    fn rebuild(
        &self,
        prev: &Self,
        (): &mut Self::ViewState,
        _ctx: &mut ViewCtx,
        mut element: Mut<'_, Self::Element>,
        _: Arg<'_, State>,
    ) {
        let mut text_area = widgets::Prose::text_mut(&mut element);

        // TODO - Replace this with properties on the Prose view
        if self.text_color != prev.text_color {
            if let Some(color) = self.text_color {
                text_area.insert_prop(ContentColor { color });
            } else {
                text_area.remove_prop::<ContentColor>();
            }
        }
        if self.disabled_text_color != prev.disabled_text_color {
            if let Some(color) = self.disabled_text_color {
                text_area.insert_prop(DisabledContentColor(ContentColor { color }));
            } else {
                text_area.remove_prop::<DisabledContentColor>();
            }
        }

        if prev.content != self.content {
            widgets::TextArea::reset_text(&mut text_area, &self.content);
        }
        if prev.text_alignment != self.text_alignment {
            widgets::TextArea::set_text_alignment(&mut text_area, self.text_alignment);
        }
        if prev.text_size != self.text_size {
            widgets::TextArea::insert_style(
                &mut text_area,
                StyleProperty::FontSize(self.text_size),
            );
        }
        if prev.weight != self.weight {
            widgets::TextArea::insert_style(&mut text_area, StyleProperty::FontWeight(self.weight));
        }
        if prev.line_break_mode != self.line_break_mode {
            widgets::TextArea::set_word_wrap(
                &mut text_area,
                self.line_break_mode == LineBreaking::WordWrap,
            );
            drop(text_area);
            widgets::Prose::set_clip(&mut element, line_break_clips(self.line_break_mode));
        }
    }

    fn teardown(&self, (): &mut Self::ViewState, _: &mut ViewCtx, _: Mut<'_, Self::Element>) {}

    fn message(
        &self,
        _view_state: &mut Self::ViewState,
        message: &mut MessageContext,
        _element: Mut<'_, Self::Element>,
        _app_state: Arg<'_, State>,
    ) -> MessageResult<Action> {
        tracing::error!(
            ?message,
            "Message arrived in Prose::message, but Prose doesn't consume any messages, this is a bug"
        );
        MessageResult::Stale
    }
}
