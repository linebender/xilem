// Copyright 2024 the Xilem Authors
// SPDX-License-Identifier: Apache-2.0

use masonry::{text2::TextBrush, widget::WidgetMut, WidgetPod};

use crate::{Color, MasonryView, MessageResult, TextAlignment, ViewCtx, ViewId};

// FIXME - A major problem of the current approach (always setting the textbox contents)
// is that if the user forgets to hook up the modify the state's contents in the callback,
// the textbox will always be reset to the initial state. This will be very annoying for the user.

type Callback<State, Action> = Box<dyn Fn(&mut State, String) -> Action + Send + Sync + 'static>;

pub fn textbox<F, State, Action>(contents: String, on_changed: F) -> Textbox<State, Action>
where
    F: Fn(&mut State, String) -> Action + Send + Sync + 'static,
{
    // TODO: Allow setting a placeholder
    Textbox {
        contents,
        on_changed: Box::new(on_changed),
        on_enter: None,
        text_brush: Color::WHITE.into(),
        alignment: TextAlignment::default(),
        disabled: false,
    }
}

pub struct Textbox<State, Action> {
    contents: String,
    on_changed: Callback<State, Action>,
    on_enter: Option<Callback<State, Action>>,
    text_brush: TextBrush,
    alignment: TextAlignment,
    disabled: bool,
    // TODO: add more attributes of `masonry::widget::Label`
}

impl<State, Action> Textbox<State, Action> {
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

    pub fn on_enter<F>(mut self, on_enter: F) -> Self
    where
        F: Fn(&mut State, String) -> Action + Send + Sync + 'static,
    {
        self.on_enter = Some(Box::new(on_enter));
        self
    }
}

impl<State: 'static, Action: 'static> MasonryView<State, Action> for Textbox<State, Action> {
    type Element = masonry::widget::Textbox;
    type ViewState = ();

    fn build(&self, cx: &mut ViewCtx) -> (WidgetPod<Self::Element>, Self::ViewState) {
        cx.with_leaf_action_widget(|_| {
            WidgetPod::new(
                masonry::widget::Textbox::new(self.contents.clone())
                    .with_text_brush(self.text_brush.clone())
                    .with_text_alignment(self.alignment),
            )
        })
    }

    fn rebuild(
        &self,
        _view_state: &mut Self::ViewState,
        cx: &mut ViewCtx,
        prev: &Self,
        mut element: WidgetMut<Self::Element>,
    ) {
        // Unlike the other properties, we don't compare to the previous value;
        // instead, we compare directly to the element's text. This is to handle
        // cases like "Previous data says contents is 'fooba', user presses 'r',
        // now data and contents are both 'foobar' but previous data is 'fooba'"
        // without calling `set_text`.
        if self.contents != element.text() {
            element.reset_text(self.contents.clone());
            cx.mark_changed();
        }

        // if prev.disabled != self.disabled {
        //     element.set_disabled(self.disabled);
        //     cx.mark_changed();
        // }
        if prev.text_brush != self.text_brush {
            element.set_text_brush(self.text_brush.clone());
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
        id_path: &[ViewId],
        message: Box<dyn std::any::Any>,
        app_state: &mut State,
    ) -> crate::MessageResult<Action> {
        debug_assert!(
            id_path.is_empty(),
            "id path should be empty in Textbox::message"
        );
        match message.downcast::<masonry::Action>() {
            Ok(action) => match *action {
                masonry::Action::TextChanged(text) => {
                    MessageResult::Action((self.on_changed)(app_state, text))
                }
                masonry::Action::TextEntered(text) if self.on_enter.is_some() => {
                    MessageResult::Action((self.on_enter.as_ref().unwrap())(app_state, text))
                }
                masonry::Action::TextEntered(_) => {
                    tracing::error!("Textbox::message: on_enter is not set");
                    MessageResult::Stale(action)
                }
                _ => {
                    tracing::error!("Wrong action type in Textbox::message: {action:?}");
                    MessageResult::Stale(action)
                }
            },
            Err(message) => {
                tracing::error!("Wrong message type in Textbox::message");
                MessageResult::Stale(message)
            }
        }
    }
}
