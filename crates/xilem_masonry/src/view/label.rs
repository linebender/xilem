use masonry::{widget::WidgetMut, ArcStr, WidgetPod};

use crate::{ChangeFlags, Color, MasonryView, MessageResult, TextAlignment, ViewCx, ViewId};

pub fn label(label: impl Into<ArcStr>) -> Label {
    Label {
        label: label.into(),
        text_color: Color::default(),
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
        (
            WidgetPod::new(
                masonry::widget::Label::new(self.label.clone())
                    .with_text_color(self.text_color)
                    .with_text_alignment(self.alignment)
                    .with_disabled(self.disabled),
            ),
            (),
        )
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
        if prev.disabled != self.disabled {
            element.set_disabled(self.disabled);
            changeflags.changed |= ChangeFlags::CHANGED.changed;
        }
        if prev.text_color != self.text_color {
            element.set_text_color(self.text_color);
            changeflags.changed |= ChangeFlags::CHANGED.changed;
        }
        if prev.alignment != self.alignment {
            element.set_text_alignment(self.alignment);
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
        MessageResult::Stale(message)
    }
}
