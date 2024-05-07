// Copyright 2024 the Xilem Authors
// SPDX-License-Identifier: Apache-2.0

use masonry::{widget::WidgetMut, ArcStr, WidgetPod};

use crate::{Color, MasonryView, MessageResult, TextAlignment, ViewCx, ViewId};

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

impl<State, Action> MasonryView<State, Action> for Label {
    type Element = masonry::widget::Label;
    type ViewState = ();

    fn build(&self, _cx: &mut ViewCx) -> (WidgetPod<Self::Element>, Self::ViewState) {
        let widget_pod = WidgetPod::new(
            masonry::widget::Label::new(self.label.clone())
                .with_text_brush(self.text_color)
                .with_text_alignment(self.alignment),
        );
        (widget_pod, ())
    }

    fn rebuild(
        &self,
        _view_state: &mut Self::ViewState,
        cx: &mut ViewCx,
        prev: &Self,
        mut element: WidgetMut<Self::Element>,
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

    fn message(
        &self,
        _view_state: &mut Self::ViewState,
        _id_path: &[ViewId],
        message: Box<dyn std::any::Any>,
        _app_state: &mut State,
    ) -> crate::MessageResult<Action> {
        tracing::error!("Message arrived in Label::message, but Label doesn't consume any messages, this is a bug");
        MessageResult::Stale(message)
    }
}
