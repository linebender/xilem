// Copyright 2024 the Xilem Authors
// SPDX-License-Identifier: Apache-2.0

use masonry::{text2::TextBrush, widget, ArcStr};
use xilem_core::Mut;

use crate::{Color, MessageResult, Pod, TextAlignment, View, ViewCtx, ViewId};

pub fn label(label: impl Into<ArcStr>) -> Label {
    Label {
        label: label.into(),
        text_brush: Color::WHITE.into(),
        alignment: TextAlignment::default(),
        text_size: masonry::theme::TEXT_SIZE_NORMAL as f32,
    }
}

pub struct Label {
    label: ArcStr,

    text_brush: TextBrush,
    alignment: TextAlignment,
    text_size: f32,
    // TODO: add more attributes of `masonry::widget::Label`
}

impl Label {
    #[doc(alias = "color")]
    pub fn brush(mut self, brush: impl Into<TextBrush>) -> Self {
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
}

impl<State, Action> View<State, Action, ViewCtx> for Label {
    type Element = Pod<widget::Label>;
    type ViewState = ();

    fn build(&self, _ctx: &mut ViewCtx) -> (Self::Element, Self::ViewState) {
        let widget_pod = Pod::new(
            widget::Label::new(self.label.clone())
                .with_text_brush(self.text_brush.clone())
                .with_text_alignment(self.alignment)
                .with_text_size(self.text_size),
        );
        (widget_pod, ())
    }

    fn rebuild<'el>(
        &self,
        prev: &Self,
        (): &mut Self::ViewState,
        ctx: &mut ViewCtx,
        mut element: Mut<'el, Self::Element>,
    ) -> Mut<'el, Self::Element> {
        if prev.label != self.label {
            element.set_text(self.label.clone());
            ctx.mark_changed();
        }
        if prev.text_brush != self.text_brush {
            element.set_text_brush(self.text_brush.clone());
            ctx.mark_changed();
        }
        if prev.alignment != self.alignment {
            element.set_alignment(self.alignment);
            ctx.mark_changed();
        }
        if prev.text_size != self.text_size {
            element.set_text_size(self.text_size);
            ctx.mark_changed();
        }
        element
    }

    fn teardown(&self, (): &mut Self::ViewState, _: &mut ViewCtx, _: Mut<'_, Self::Element>) {}

    fn message(
        &self,
        (): &mut Self::ViewState,
        _id_path: &[ViewId],
        message: xilem_core::DynMessage,
        _app_state: &mut State,
    ) -> crate::MessageResult<Action> {
        tracing::error!("Message arrived in Label::message, but Label doesn't consume any messages, this is a bug");
        MessageResult::Stale(message)
    }
}
