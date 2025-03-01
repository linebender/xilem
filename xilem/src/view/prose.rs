// Copyright 2024 the Xilem Authors
// SPDX-License-Identifier: Apache-2.0

use masonry::core::{ArcStr, StyleProperty};
use masonry::widgets::{
    LineBreaking, {self},
};
use vello::peniko::Brush;

use crate::core::{DynMessage, Mut, ViewMarker};
use crate::{Color, MessageResult, Pod, TextAlignment, View, ViewCtx, ViewId};

pub fn prose(content: impl Into<ArcStr>) -> Prose {
    Prose {
        content: content.into(),
        text_brush: Color::WHITE.into(),
        alignment: TextAlignment::default(),
        text_size: masonry::theme::TEXT_SIZE_NORMAL,
        line_break_mode: LineBreaking::WordWrap,
    }
}

/// A version of [`prose`] suitable for including in the same line
/// as other content.
///
/// Note that setting [`alignment`](Prose::alignment) on the result
/// will be meaningless.
#[doc(alias = "span")]
pub fn inline_prose(content: impl Into<ArcStr>) -> Prose {
    prose(content).line_break_mode(LineBreaking::Overflow)
}

#[must_use = "View values do nothing unless provided to Xilem."]
pub struct Prose {
    content: ArcStr,

    text_brush: Brush,
    alignment: TextAlignment,
    text_size: f32,
    line_break_mode: LineBreaking,
    // TODO: disabled: bool,
    // TODO: add more attributes of `masonry::widgets::Prose`
}

impl Prose {
    #[doc(alias = "color")]
    pub fn brush(mut self, brush: impl Into<Brush>) -> Self {
        self.text_brush = brush.into();
        self
    }

    pub fn alignment(mut self, alignment: TextAlignment) -> Self {
        self.alignment = alignment;
        self
    }

    #[doc(alias = "font_size")]
    pub fn text_size(mut self, text_size: f32) -> Self {
        self.text_size = text_size;
        self
    }
    pub fn line_break_mode(mut self, line_break_mode: LineBreaking) -> Self {
        self.line_break_mode = line_break_mode;
        self
    }
}

fn line_break_clips(linebreaking: LineBreaking) -> bool {
    matches!(linebreaking, LineBreaking::Clip | LineBreaking::WordWrap)
}

impl ViewMarker for Prose {}
impl<State, Action> View<State, Action, ViewCtx> for Prose {
    type Element = Pod<widgets::Prose>;
    type ViewState = ();

    fn build(&self, ctx: &mut ViewCtx) -> (Self::Element, Self::ViewState) {
        let text_area = widgets::TextArea::new_immutable(&self.content)
            .with_brush(self.text_brush.clone())
            .with_alignment(self.alignment)
            .with_style(StyleProperty::FontSize(self.text_size))
            .with_word_wrap(self.line_break_mode == LineBreaking::WordWrap);
        let widget_pod = ctx.new_pod(
            widgets::Prose::from_text_area(text_area)
                .with_clip(line_break_clips(self.line_break_mode)),
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
        let mut text_area = widgets::Prose::text_mut(&mut element);
        if prev.content != self.content {
            widgets::TextArea::reset_text(&mut text_area, &self.content);
        }
        if prev.text_brush != self.text_brush {
            widgets::TextArea::set_brush(&mut text_area, self.text_brush.clone());
        }
        if prev.alignment != self.alignment {
            widgets::TextArea::set_alignment(&mut text_area, self.alignment);
        }
        if prev.text_size != self.text_size {
            widgets::TextArea::insert_style(
                &mut text_area,
                StyleProperty::FontSize(self.text_size),
            );
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

    fn teardown(&self, (): &mut Self::ViewState, _: &mut ViewCtx, _: Mut<Self::Element>) {}

    fn message(
        &self,
        _view_state: &mut Self::ViewState,
        _id_path: &[ViewId],
        message: DynMessage,
        _app_state: &mut State,
    ) -> MessageResult<Action> {
        tracing::error!(
            "Message arrived in Prose::message, but Prose doesn't consume any messages, this is a bug"
        );
        MessageResult::Stale(message)
    }
}
