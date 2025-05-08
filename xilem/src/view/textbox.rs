// Copyright 2024 the Xilem Authors
// SPDX-License-Identifier: Apache-2.0

use masonry::widgets;
use vello::peniko::Brush;

use crate::core::{DynMessage, Mut, View, ViewMarker};
use crate::{Color, InsertNewline, MessageResult, Pod, TextAlignment, ViewCtx, ViewId};

// FIXME - A major problem of the current approach (always setting the textbox contents)
// is that if the user forgets to hook up the modify the state's contents in the callback,
// the textbox will always be reset to the initial state. This will be very annoying for the user.

type Callback<State, Action> = Box<dyn Fn(&mut State, String) -> Action + Send + Sync + 'static>;

/// A view which displays editable text.
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
        insert_newline: InsertNewline::default(),
        // TODO?: disabled: false,
    }
}

/// The [`View`] created by [`textbox`].
#[must_use = "View values do nothing unless provided to Xilem."]
pub struct Textbox<State, Action> {
    contents: String,
    on_changed: Callback<State, Action>,
    on_enter: Option<Callback<State, Action>>,
    text_brush: Brush,
    alignment: TextAlignment,
    insert_newline: InsertNewline,
    // TODO: add more attributes of `masonry::widgets::TextBox`
}

impl<State, Action> Textbox<State, Action> {
    /// Set the brush used to paint the text.
    #[doc(alias = "color")]
    pub fn brush(mut self, color: impl Into<Brush>) -> Self {
        self.text_brush = color.into();
        self
    }

    /// Set the [alignment](https://en.wikipedia.org/wiki/Typographic_alignment) of the text.
    pub fn alignment(mut self, alignment: TextAlignment) -> Self {
        self.alignment = alignment;
        self
    }

    /// Configures how this text area handles the user pressing Enter <kbd>↵</kbd>.
    pub fn insert_newline(mut self, insert_newline: InsertNewline) -> Self {
        self.insert_newline = insert_newline;
        self
    }

    /// Set a callback that will be run when the user presses the `Enter` key to submit their input.
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
    type Element = Pod<widgets::Textbox>;
    type ViewState = ();

    fn build(&self, ctx: &mut ViewCtx) -> (Self::Element, Self::ViewState) {
        // TODO: Maybe we want a shared TextArea View?
        let text_area = widgets::TextArea::new_editable(&self.contents)
            .with_brush(self.text_brush.clone())
            .with_alignment(self.alignment)
            .with_insert_newline(self.insert_newline);
        let textbox = widgets::Textbox::from_text_area(text_area);

        // Ensure that the actions from the *inner* TextArea get routed correctly.
        let id = textbox.area_pod().id();
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
        let mut text_area = widgets::Textbox::text_mut(&mut element);

        // Unlike the other properties, we don't compare to the previous value;
        // instead, we compare directly to the element's text. This is to handle
        // cases like "Previous data says contents is 'fooba', user presses 'r',
        // now data and contents are both 'foobar' but previous data is 'fooba'"
        // without calling `set_text`.

        // This is probably not the right behaviour, but determining what is the right behaviour is hard
        if text_area.widget.text() != &self.contents {
            widgets::TextArea::reset_text(&mut text_area, &self.contents);
        }

        if prev.text_brush != self.text_brush {
            widgets::TextArea::set_brush(&mut text_area, self.text_brush.clone());
        }
        if prev.alignment != self.alignment {
            widgets::TextArea::set_alignment(&mut text_area, self.alignment);
        }
        if prev.insert_newline != self.insert_newline {
            widgets::TextArea::set_insert_newline(&mut text_area, self.insert_newline);
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
        match message.downcast::<masonry::core::DefaultAction>() {
            Ok(action) => match *action {
                masonry::core::DefaultAction::TextChanged(text) => {
                    MessageResult::Action((self.on_changed)(app_state, text))
                }
                masonry::core::DefaultAction::TextEntered(text) if self.on_enter.is_some() => {
                    MessageResult::Action((self.on_enter.as_ref().unwrap())(app_state, text))
                }
                masonry::core::DefaultAction::TextEntered(_) => {
                    tracing::error!("Textbox::message: on_enter is not set");
                    MessageResult::Stale(DynMessage(action))
                }
                _ => {
                    tracing::error!("Wrong action type in Textbox::message: {action:?}");
                    MessageResult::Stale(DynMessage(action))
                }
            },
            Err(message) => {
                tracing::error!("Wrong message type in Textbox::message");
                MessageResult::Stale(message)
            }
        }
    }
}
