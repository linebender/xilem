// Copyright 2024 the Xilem Authors
// SPDX-License-Identifier: Apache-2.0

use masonry::widget;
use vello::peniko::Brush;

use crate::core::{DynMessage, Mut, View, ViewMarker};
use crate::{Color, MessageResult, Pod, TextAlignment, ViewCtx, ViewId};

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
        // TODO?: disabled: false,
    }
}

#[must_use = "View values do nothing unless provided to Xilem."]
pub struct Textbox<State, Action> {
    contents: String,
    on_changed: Callback<State, Action>,
    on_enter: Option<Callback<State, Action>>,
    text_brush: Brush,
    alignment: TextAlignment,
    // TODO: add more attributes of `masonry::widget::TextBox`
}

impl<State, Action> Textbox<State, Action> {
    #[doc(alias = "color")]
    pub fn brush(mut self, color: impl Into<Brush>) -> Self {
        self.text_brush = color.into();
        self
    }

    pub fn alignment(mut self, alignment: TextAlignment) -> Self {
        self.alignment = alignment;
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

impl<State, Action> ViewMarker for Textbox<State, Action> {}
impl<State: 'static, Action: 'static> View<State, Action, ViewCtx> for Textbox<State, Action> {
    type Element = Pod<widget::Textbox>;
    type ViewState = ();

    fn build(&self, ctx: &mut ViewCtx) -> (Self::Element, Self::ViewState) {
        // TODO: Maybe we want a shared TextRegion View?
        let text_region = widget::TextRegion::new_editable(&self.contents)
            .with_brush(self.text_brush.clone())
            .with_alignment(self.alignment);
        let textbox = widget::Textbox::from_text_region(text_region);

        // Ensure that the actions from the *inner* textregion get routed correctly.
        let id = textbox.region_pod().id();
        ctx.record_action(id);
        let widget_pod = ctx.new_pod(textbox);
        (widget_pod, ())
    }

    fn rebuild(
        &self,
        prev: &Self,
        _: &mut Self::ViewState,
        _ctx: &mut ViewCtx,
        mut element: Mut<Self::Element>,
    ) {
        let mut region = widget::Textbox::text_mut(&mut element);

        // Unlike the other properties, we don't compare to the previous value;
        // instead, we compare directly to the element's text. This is to handle
        // cases like "Previous data says contents is 'fooba', user presses 'r',
        // now data and contents are both 'foobar' but previous data is 'fooba'"
        // without calling `set_text`.

        // This is probably not the right behaviour, but determining what is the right behaviour is hard
        if self.contents != region.widget.text() {
            widget::TextRegion::reset_text(&mut region, &self.contents);
        }

        if prev.text_brush != self.text_brush {
            widget::TextRegion::set_brush(&mut region, self.text_brush.clone());
        }
        if prev.alignment != self.alignment {
            widget::TextRegion::set_alignment(&mut region, self.alignment);
        }
    }

    fn teardown(&self, _: &mut Self::ViewState, ctx: &mut ViewCtx, element: Mut<Self::Element>) {
        ctx.teardown_leaf(element);
    }

    fn message(
        &self,
        _: &mut Self::ViewState,
        id_path: &[ViewId],
        message: DynMessage,
        app_state: &mut State,
    ) -> MessageResult<Action> {
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
