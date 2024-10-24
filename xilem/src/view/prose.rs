// Copyright 2024 the Xilem Authors
// SPDX-License-Identifier: Apache-2.0

use masonry::text::{ArcStr, TextBrush};
use masonry::widget;

use crate::core::{DynMessage, Mut, ViewMarker};
use crate::{Color, MessageResult, Pod, TextAlignment, View, ViewCtx, ViewId};

pub fn prose(content: impl Into<ArcStr>) -> Prose {
    Prose {
        content: content.into(),
        text_brush: Color::WHITE.into(),
        alignment: TextAlignment::default(),
        text_size: masonry::theme::TEXT_SIZE_NORMAL as f32,
    }
}

pub struct Prose {
    content: ArcStr,

    text_brush: TextBrush,
    alignment: TextAlignment,
    text_size: f32,
    // TODO: disabled: bool,
    // TODO: add more attributes of `masonry::widget::Prose`
}

impl Prose {
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

impl ViewMarker for Prose {}
impl<State, Action> View<State, Action, ViewCtx> for Prose {
    type Element = Pod<widget::Prose>;
    type ViewState = ();

    fn build(&self, ctx: &mut ViewCtx) -> (Self::Element, Self::ViewState) {
        let widget_pod = ctx.new_pod(
            widget::Prose::new(self.content.clone())
                .with_text_brush(self.text_brush.clone())
                .with_text_alignment(self.alignment)
                .with_text_size(self.text_size),
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
        if prev.content != self.content {
            widget::Prose::set_text(&mut element, self.content.clone());
        }
        if prev.text_brush != self.text_brush {
            widget::Prose::set_text_brush(&mut element, self.text_brush.clone());
        }
        if prev.alignment != self.alignment {
            widget::Prose::set_alignment(&mut element, self.alignment);
        }
        if prev.text_size != self.text_size {
            widget::Prose::set_text_size(&mut element, self.text_size);
        }
    }

    fn teardown(&self, (): &mut Self::ViewState, _: &mut ViewCtx, _: Mut<Self::Element>) {}

    fn message(
        &self,
        _view_state: &mut Self::ViewState,
        _id_path: &[ViewId],
        message: DynMessage,
        _app_state: &mut State,
    ) -> crate::MessageResult<Action> {
        tracing::error!("Message arrived in Prose::message, but Prose doesn't consume any messages, this is a bug");
        MessageResult::Stale(message)
    }
}
