// Copyright 2024 the Xilem Authors
// SPDX-License-Identifier: Apache-2.0

use masonry::{
    widget::{self, WidgetMut},
    ArcStr, WidgetPod,
};

use crate::{Color, MessageResult, Pod, TextAlignment, View, ViewCtx, ViewId};

pub fn label(label: impl Into<ArcStr>) -> Label {
    Label {
        label: label.into(),
        text_color: Color::WHITE,
        alignment: TextAlignment::default(),
        disabled: false,
    }
}

pub struct Label {
    label: ArcStr,
    text_color: Color,
    alignment: TextAlignment,
    disabled: bool,
    // TODO: add more attributes of `masonry::widget::Label`
}

impl Label {
    pub fn color(mut self, color: Color) -> Self {
        self.text_color = color;
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

impl<State, Action> View<State, Action, ViewCtx> for Label {
    type Element = Pod<widget::Label>;
    type ViewState = ();

    fn build(&self, _cx: &mut ViewCtx) -> (Self::Element, Self::ViewState) {
        let widget_pod = WidgetPod::new(
            widget::Label::new(self.label.clone())
                .with_text_brush(self.text_color)
                .with_text_alignment(self.alignment),
        );
        (widget_pod.into(), ())
    }

    fn rebuild(
        &self,
        prev: &Self,
        (): &mut Self::ViewState,
        cx: &mut ViewCtx,
        mut element: WidgetMut<'_, widget::Label>,
    ) {
        if prev.label != self.label {
            element.set_text(self.label.clone());
            cx.mark_changed();
        }
        // if prev.disabled != self.disabled {
        //     element.set_disabled(self.disabled);
        //     cx.mark_changed();
        // }
        if prev.text_color != self.text_color {
            element.set_text_brush(self.text_color);
            cx.mark_changed();
        }
        if prev.alignment != self.alignment {
            element.set_alignment(self.alignment);
            cx.mark_changed();
        }
    }

    fn teardown(&self, (): &mut Self::ViewState, _: &mut ViewCtx, _: WidgetMut<'_, widget::Label>) {
    }

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
