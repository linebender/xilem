// Copyright 2024 the Xilem Authors
// SPDX-License-Identifier: Apache-2.0

use masonry::{text2::TextBrush, widget::WidgetMut, WidgetPod};

use crate::{ChangeFlags, Color, MasonryView, MessageResult, TextAlignment, ViewCx, ViewId};

pub fn textbox() -> Textbox {
    // TODO: Allow setting a placeholder
    Textbox {
        text_brush: Color::WHITE.into(),
        alignment: TextAlignment::default(),
        disabled: false,
    }
}

pub struct Textbox {
    text_brush: TextBrush,
    alignment: TextAlignment,
    disabled: bool,
    // TODO: add more attributes of `masonry::widget::Label`
}

impl Textbox {
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

impl<State, Action> MasonryView<State, Action> for Textbox {
    type Element = masonry::widget::Textbox<String>;
    type ViewState = ();

    fn build(&self, _cx: &mut ViewCx) -> (WidgetPod<Self::Element>, Self::ViewState) {
        let widget_pod = WidgetPod::new(
            masonry::widget::Textbox::new(String::new())
                .with_text_brush(self.text_brush.clone())
                .with_text_alignment(self.alignment),
        );
        (widget_pod, ())
    }

    fn rebuild(
        &self,
        _view_state: &mut Self::ViewState,
        _cx: &mut ViewCx,
        prev: &Self,
        mut element: WidgetMut<Self::Element>,
    ) -> crate::ChangeFlags {
        let mut changeflags = ChangeFlags::UNCHANGED;

        // if prev.disabled != self.disabled {
        //     element.set_disabled(self.disabled);
        //     changeflags.changed |= ChangeFlags::CHANGED.changed;
        // }
        if prev.text_brush != self.text_brush {
            element.set_text_brush(self.text_brush.clone());
            changeflags.changed |= ChangeFlags::CHANGED.changed;
        }
        if prev.alignment != self.alignment {
            element.set_alignment(self.alignment);
            changeflags.changed |= ChangeFlags::CHANGED.changed;
        }
        changeflags
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
