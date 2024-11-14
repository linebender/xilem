// Copyright 2024 the Xilem Authors
// SPDX-License-Identifier: Apache-2.0

use masonry::text::{ArcStr, TextBrush};
use masonry::widget;

use crate::core::{DynMessage, Mut, ViewMarker};
use crate::{Color, MessageResult, Pod, TextAlignment, TextWeight, View, ViewCtx, ViewId};

pub fn label(label: impl Into<ArcStr>) -> Label {
    Label {
        label: label.into(),
        text_brush: Color::WHITE.into(),
        alignment: TextAlignment::default(),
        text_size: masonry::theme::TEXT_SIZE_NORMAL,
        weight: TextWeight::NORMAL,
    }
}

#[must_use = "View values do nothing unless provided to Xilem."]
pub struct Label {
    label: ArcStr,

    text_brush: TextBrush,
    alignment: TextAlignment,
    text_size: f32,
    weight: TextWeight,
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

    pub fn weight(mut self, weight: TextWeight) -> Self {
        self.weight = weight;
        self
    }
}

impl ViewMarker for Label {}
impl<State, Action> View<State, Action, ViewCtx> for Label {
    type Element = Pod<widget::Label>;
    type ViewState = ();

    fn build(&self, ctx: &mut ViewCtx) -> (Self::Element, Self::ViewState) {
        let widget_pod = ctx.new_pod(
            widget::Label::new(self.label.clone())
                .with_text_brush(self.text_brush.clone())
                .with_text_alignment(self.alignment)
                .with_text_size(self.text_size)
                .with_weight(self.weight),
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
            widget::Label::set_text(&mut element, self.label.clone());
        }
        if prev.text_brush != self.text_brush {
            widget::Label::set_brush(&mut element, self.text_brush.clone());
        }
        if prev.alignment != self.alignment {
            widget::Label::set_alignment(&mut element, self.alignment);
        }
        if prev.text_size != self.text_size {
            widget::Label::set_text_size(&mut element, self.text_size);
        }
        if prev.weight != self.weight {
            widget::Label::set_weight(&mut element, self.weight);
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
