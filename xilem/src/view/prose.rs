// Copyright 2024 the Xilem Authors
// SPDX-License-Identifier: Apache-2.0

use masonry::{
    text2::TextBrush,
    widget::{self, WidgetMut},
    ArcStr, WidgetPod,
};

use crate::{Color, MessageResult, Pod, TextAlignment, View, ViewCtx, ViewId};

pub fn prose(label: impl Into<ArcStr>) -> Prose {
    Prose {
        label: label.into(),
        text_brush: Color::WHITE.into(),
        alignment: TextAlignment::default(),
        disabled: false,
    }
}

pub struct Prose {
    label: ArcStr,
    text_brush: TextBrush,
    alignment: TextAlignment,
    disabled: bool,
    // TODO: add more attributes of `masonry::widget::Label`
}

impl Prose {
    #[doc(alias = "color")]
    pub fn brush(mut self, color: impl Into<TextBrush>) -> Self {
        self.text_brush = color.into();
        self
    }

    pub fn alignment(mut self, alignment: TextAlignment) -> Self {
        self.alignment = alignment;
        self
    }

    pub fn disabled(mut self) -> Self {
        self.disabled = true;
        self
    }
}

impl<State, Action> View<State, Action, ViewCtx> for Prose {
    type Element = Pod<widget::Prose>;
    type ViewState = ();

    fn build(&self, _ctx: &mut ViewCtx) -> (Self::Element, Self::ViewState) {
        let widget_pod = WidgetPod::new(
            widget::Prose::new(self.label.clone())
                .with_text_brush(self.text_brush.clone())
                .with_text_alignment(self.alignment),
        );
        (widget_pod.into(), ())
    }

    fn rebuild(
        &self,
        prev: &Self,
        (): &mut Self::ViewState,
        ctx: &mut ViewCtx,
        mut element: WidgetMut<'_, widget::Prose>,
    ) {
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
    }

    fn teardown(&self, (): &mut Self::ViewState, _: &mut ViewCtx, _: WidgetMut<'_, widget::Prose>) {
    }

    fn message(
        &self,
        _view_state: &mut Self::ViewState,
        _id_path: &[ViewId],
        message: xilem_core::DynMessage,
        _app_state: &mut State,
    ) -> crate::MessageResult<Action> {
        tracing::error!("Message arrived in Prose::message, but Prose doesn't consume any messages, this is a bug");
        MessageResult::Stale(message)
    }
}
