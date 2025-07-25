// Copyright 2024 the Xilem Authors
// SPDX-License-Identifier: Apache-2.0

use masonry::core::{NewWidget, Properties, WidgetId, WidgetOptions};
use masonry::properties::{
    Background, BorderColor, BorderWidth, BoxShadow, CornerRadius, DisabledBackground,
    DisabledTextColor, Padding, TextColor,
};
use masonry::widgets::{self, TextAction};
use vello::kurbo::Affine;
use vello::peniko::Color;

use crate::core::{MessageContext, Mut, View, ViewMarker};
use crate::property_tuple::PropertyTuple;
use crate::style::Style;
use crate::{InsertNewline, MessageResult, Pod, TextAlign, ViewCtx};

// FIXME - A major problem of the current approach (always setting the text_input contents)
// is that if the user forgets to hook up the modify the state's contents in the callback,
// the text_input will always be reset to the initial state. This will be very annoying for the user.

type Callback<State, Action> = Box<dyn Fn(&mut State, String) -> Action + Send + Sync + 'static>;

/// A view which displays editable text.
pub fn text_input<F, State, Action>(contents: String, on_changed: F) -> TextInput<State, Action>
where
    F: Fn(&mut State, String) -> Action + Send + Sync + 'static,
{
    // TODO: Allow setting a placeholder
    TextInput {
        contents,
        on_changed: Box::new(on_changed),
        on_enter: None,
        text_color: None,
        disabled_text_color: None,
        text_alignment: TextAlign::default(),
        insert_newline: InsertNewline::default(),
        disabled: false,
        properties: TextInputProps::default(),
    }
}

/// The [`View`] created by [`text_input`].
#[must_use = "View values do nothing unless provided to Xilem."]
pub struct TextInput<State, Action> {
    contents: String,
    on_changed: Callback<State, Action>,
    on_enter: Option<Callback<State, Action>>,
    text_color: Option<Color>,
    disabled_text_color: Option<Color>,
    text_alignment: TextAlign,
    insert_newline: InsertNewline,
    disabled: bool,
    properties: TextInputProps,
    // TODO: add more attributes of `masonry::widgets::TextInput`
}

impl<State, Action> TextInput<State, Action> {
    /// Set the text's color.
    ///
    /// This overwrites the default `TextColor` property for the inner `TextArea` widget.
    pub fn text_color(mut self, color: Color) -> Self {
        self.text_color = Some(color);
        self
    }

    /// Set the text's color when the text input is disabled.
    ///
    /// This overwrites the default `DisabledTextColor` property for the inner `TextArea` widget.
    pub fn disabled_text_color(mut self, color: Color) -> Self {
        self.disabled_text_color = Some(color);
        self
    }

    /// Set the [text alignment](https://en.wikipedia.org/wiki/Typographic_alignment) of the text.
    pub fn text_alignment(mut self, text_alignment: TextAlign) -> Self {
        self.text_alignment = text_alignment;
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

    /// Set the disabled state of the widget.
    pub fn disabled(mut self, disabled: bool) -> Self {
        self.disabled = disabled;
        self
    }
}

impl<S, A> Style for TextInput<S, A> {
    type Props = TextInputProps;

    fn properties(&mut self) -> &mut Self::Props {
        &mut self.properties
    }
}

crate::declare_property_tuple!(
    pub TextInputProps;
    TextInput<S, A>;

    Background, 0;
    DisabledBackground, 1;
    BorderColor, 2;
    BorderWidth, 3;
    BoxShadow, 4;
    CornerRadius, 5;
    Padding, 6;
);

impl<State, Action> ViewMarker for TextInput<State, Action> {}
impl<State: 'static, Action: 'static> View<State, Action, ViewCtx> for TextInput<State, Action> {
    type Element = Pod<widgets::TextInput>;
    type ViewState = ();

    fn build(&self, ctx: &mut ViewCtx, _: &mut State) -> (Self::Element, Self::ViewState) {
        // TODO: Maybe we want a shared TextArea View?
        let text_area = widgets::TextArea::new_editable(&self.contents)
            .with_text_alignment(self.text_alignment)
            .with_insert_newline(self.insert_newline);

        // TODO - Replace this with properties on the TextInput view
        // once we implement property inheritance or something like it.
        let mut props = Properties::new();
        if let Some(color) = self.text_color {
            props.insert(TextColor { color });
        }
        if let Some(color) = self.disabled_text_color {
            props.insert(DisabledTextColor(TextColor { color }));
        }

        let text_input = widgets::TextInput::from_text_area(NewWidget::new_with(
            text_area,
            WidgetId::next(),
            WidgetOptions {
                disabled: self.disabled,
                transform: Affine::default(),
            },
            props,
        ));

        // Ensure that the actions from the *inner* TextArea get routed correctly.
        let id = text_input.area_pod().id();
        ctx.record_action(id);
        let mut pod = ctx.create_pod(text_input);
        pod.new_widget.properties = self.properties.build_properties();
        (pod, ())
    }

    fn rebuild(
        &self,
        prev: &Self,
        _: &mut Self::ViewState,
        _ctx: &mut ViewCtx,
        mut element: Mut<'_, Self::Element>,
        _: &mut State,
    ) {
        self.properties
            .rebuild_properties(&prev.properties, &mut element);

        // TODO - Replace this with properties on the TextInput view
        if self.text_color != prev.text_color {
            if let Some(color) = self.text_color {
                element.insert_prop(TextColor { color });
            } else {
                element.remove_prop::<TextColor>();
            }
        }
        if self.disabled_text_color != prev.disabled_text_color {
            if let Some(color) = self.disabled_text_color {
                element.insert_prop(DisabledTextColor(TextColor { color }));
            } else {
                element.remove_prop::<DisabledTextColor>();
            }
        }

        if element.ctx.is_disabled() != self.disabled {
            element.ctx.set_disabled(self.disabled);
        }

        let mut text_area = widgets::TextInput::text_mut(&mut element);

        // Unlike the other properties, we don't compare to the previous value;
        // instead, we compare directly to the element's text. This is to handle
        // cases like "Previous data says contents is 'fooba', user presses 'r',
        // now data and contents are both 'foobar' but previous data is 'fooba'"
        // without calling `set_text`.

        // This is probably not the right behaviour, but determining what is the right behaviour is hard
        if text_area.widget.text() != &self.contents {
            widgets::TextArea::reset_text(&mut text_area, &self.contents);
        }

        if prev.text_alignment != self.text_alignment {
            widgets::TextArea::set_text_alignment(&mut text_area, self.text_alignment);
        }
        if prev.insert_newline != self.insert_newline {
            widgets::TextArea::set_insert_newline(&mut text_area, self.insert_newline);
        }
    }

    fn teardown(
        &self,
        _: &mut Self::ViewState,
        ctx: &mut ViewCtx,
        element: Mut<'_, Self::Element>,
        _: &mut State,
    ) {
        ctx.teardown_leaf(element);
    }

    fn message(
        &self,
        _: &mut Self::ViewState,
        message: &mut MessageContext,
        _: Mut<'_, Self::Element>,
        app_state: &mut State,
    ) -> MessageResult<Action> {
        debug_assert!(
            message.remaining_path().is_empty(),
            "id path should be empty in TextInput::message"
        );
        match message.take_message::<TextAction>() {
            Some(action) => match *action {
                TextAction::Changed(text) => {
                    MessageResult::Action((self.on_changed)(app_state, text))
                }
                TextAction::Entered(text) if self.on_enter.is_some() => {
                    MessageResult::Action((self.on_enter.as_ref().unwrap())(app_state, text))
                }

                TextAction::Entered(_) => {
                    tracing::error!("Textbox::message: on_enter is not set");
                    MessageResult::Stale
                }
            },
            None => {
                tracing::error!(?message, "Wrong message type in TextInput::message");
                MessageResult::Stale
            }
        }
    }
}
