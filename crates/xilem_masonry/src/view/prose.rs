use masonry::{widget::WidgetMut, ArcStr, WidgetPod};

use crate::{ChangeFlags, Color, MasonryView, MessageResult, TextAlignment, ViewCx, ViewId};

pub fn prose(label: impl Into<ArcStr>) -> Prose {
    Prose {
        label: label.into(),
        text_color: Color::WHITE,
        alignment: TextAlignment::default(),
        disabled: false,
    }
}

pub struct Prose {
    label: ArcStr,
    text_color: Color,
    alignment: TextAlignment,
    disabled: bool,
    // TODO: add more attributes of `masonry::widget::Label`
}

impl Prose {
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

impl<State, Action> MasonryView<State, Action> for Prose {
    type Element = masonry::widget::Prose<ArcStr>;
    type ViewState = ();

    fn build(&self, _cx: &mut ViewCx) -> (WidgetPod<Self::Element>, Self::ViewState) {
        let widget_pod = WidgetPod::new(
            masonry::widget::Prose::new(self.label.clone())
                .with_text_color(self.text_color)
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

        if prev.label != self.label {
            element.set_text(self.label.clone());
            changeflags.changed |= ChangeFlags::CHANGED.changed;
        }
        // if prev.disabled != self.disabled {
        //     element.set_disabled(self.disabled);
        //     changeflags.changed |= ChangeFlags::CHANGED.changed;
        // }
        if prev.text_color != self.text_color {
            element.set_text_color(self.text_color);
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
