// Copyright 2024 the Xilem Authors
// SPDX-License-Identifier: Apache-2.0

use masonry::core::{ArcStr, NewWidget, Properties};
use masonry::properties::{
    CaretColor, ContentColor, DisabledContentColor, PlaceholderColor, SelectionColor,
    UnfocusedSelectionColor,
};
use masonry::widgets::{self, TextAction};
use vello::peniko::Color;

use crate::core::{MessageContext, Mut, View, ViewMarker};
use crate::view::Prop;
use crate::{InsertNewline, MessageResult, Pod, TextAlign, ViewCtx, WidgetView as _};

// FIXME - A major problem of the current approach (always setting the text_input contents)
// is that if the user forgets to hook up the modify the state's contents in the callback,
// the text_input will always be reset to the initial state. This will be very annoying for the user.

type Callback<State, Action> = Box<dyn Fn(&mut State, String) -> Action + Send + Sync + 'static>;

/// A view which displays editable text.
///
/// The text_input stores the content as a string, that can be set to a variable for
/// getting/setting the text_input's content from outside it's own logic. It also
/// needs to be expilicty told how to handle newlines, via the [`insert_newline`] function.
///
/// # Examples
/// Create a basic text input with it's content stored in the app state.
/// ```
/// use xilem::view::text_input;
/// # use xilem::WidgetView;
///
/// #[derive(Default)]
/// struct State {
///     content: String,
/// }
///
/// # fn view() -> impl WidgetView<State> {
/// text_input(state.content.clone(), |local_state: &mut State, input: String| {
///     local_state.buffer = input
/// })
/// # }
/// ```
///
/// Create a `text_input` that can hanle inputting a newline when enter is pressed.
/// ```
/// use xilem::view::text_input;
/// # use xilem::WidgetView;
///
/// #[derive(Default)]
/// struct State {
///     content: String,
/// }
///
/// # fn view() -> impl WidgetView<State> {
/// text_input(state.content.clone(), |local_state: &mut State, input: String| {
///     local_state.content = input
/// })
/// .insert_newline(InsertNewline::OnEnter)
/// # }
/// ```
pub fn text_input<F, State, Action>(contents: String, on_changed: F) -> TextInput<State, Action>
where
    F: Fn(&mut State, String) -> Action + Send + Sync + 'static,
{
    TextInput {
        contents,
        on_changed: Box::new(on_changed),
        on_enter: None,
        text_color: None,
        disabled_text_color: None,
        placeholder: ArcStr::default(),
        text_alignment: TextAlign::default(),
        insert_newline: InsertNewline::default(),
        disabled: false,
        // Since we don't support setting the word wrapping, we can default to
        // not clipping
        clip: true,
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
    placeholder: ArcStr,
    text_alignment: TextAlign,
    insert_newline: InsertNewline,
    disabled: bool,
    clip: bool,
    // TODO: add more attributes of `masonry::widgets::TextInput`
}

impl<State: 'static, Action: 'static> TextInput<State, Action> {
    /// Set the text's color.
    ///
    /// This overwrites the default `ContentColor` property for the inner `TextArea` widget.
    pub fn text_color(mut self, color: Color) -> Self {
        self.text_color = Some(color);
        self
    }

    /// Set the text's color when the text input is disabled.
    ///
    /// This overwrites the default `DisabledContentColor` property for the inner `TextArea` widget.
    pub fn disabled_text_color(mut self, color: Color) -> Self {
        self.disabled_text_color = Some(color);
        self
    }

    /// Set the insertion caret's color.
    ///
    /// This overwrites the default `CaretColor` property for the inner `TextArea` widget.
    pub fn caret_color(self, color: Color) -> Prop<CaretColor, Self, State, Action> {
        self.prop(CaretColor { color })
    }

    /// Set the selection's color.
    ///
    /// This overwrites the default `SelectionColor` property for the inner `TextArea` widget.
    pub fn selection_color(self, color: Color) -> Prop<SelectionColor, Self, State, Action> {
        self.prop(SelectionColor { color })
    }

    /// Set the selection's color when the window is unfocused.
    ///
    /// This overwrites the default `UnfocusedSelectionColor` property for the inner `TextArea` widget.
    pub fn unfocused_selection_color(
        self,
        color: Color,
    ) -> Prop<UnfocusedSelectionColor, Self, State, Action> {
        self.prop(UnfocusedSelectionColor(SelectionColor { color }))
    }

    /// Set the string which is shown when the input is empty.
    pub fn placeholder(mut self, placeholder_text: impl Into<ArcStr>) -> Self {
        self.placeholder = placeholder_text.into();
        self
    }

    /// Set the [`PlaceholderColor`] property, which sets the color of the text shown when the input is empty.
    pub fn placeholder_color(self, color: Color) -> Prop<PlaceholderColor, Self, State, Action> {
        self.prop(PlaceholderColor::new(color))
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

    /// Set whether the contained text will be clipped to the box if it overflows.
    ///
    /// Please note:
    /// 1) We don't currently support scrolling within a text area, so this can make some content
    ///    unviewable (without the user adding spaces and/or copy/pasting to extract content).
    ///    You should probably set this to false for small text inputs (and probably also lower
    ///    the default padding).
    /// 2) This view currently always uses word wrapping, so if there are any linebreaking
    ///    opportunities in the text, they will be taken.
    ///
    /// The default value is true (i.e. clipping is enabled).
    pub fn clip(mut self, clip: bool) -> Self {
        self.clip = clip;
        self
    }
}

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
            props.insert(ContentColor { color });
        }
        if let Some(color) = self.disabled_text_color {
            props.insert(DisabledContentColor(ContentColor { color }));
        }

        let text_input =
            widgets::TextInput::from_text_area(NewWidget::new_with_props(text_area, props))
                .with_clip(self.clip)
                .with_placeholder(self.placeholder.clone());

        // Ensure that the actions from the *inner* TextArea get routed correctly.
        let id = text_input.area_pod().id();
        ctx.record_action(id);

        let mut pod = ctx.create_pod(text_input);
        pod.new_widget.options.disabled = self.disabled;
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
        // TODO - Replace this with properties on the TextInput view
        if self.text_color != prev.text_color {
            if let Some(color) = self.text_color {
                element.insert_prop(ContentColor { color });
            } else {
                element.remove_prop::<ContentColor>();
            }
        }
        if self.disabled_text_color != prev.disabled_text_color {
            if let Some(color) = self.disabled_text_color {
                element.insert_prop(DisabledContentColor(ContentColor { color }));
            } else {
                element.remove_prop::<DisabledContentColor>();
            }
        }
        if self.placeholder != prev.placeholder {
            widgets::TextInput::set_placeholder(&mut element, self.placeholder.clone());
        }

        if prev.disabled != self.disabled {
            element.ctx.set_disabled(self.disabled);
        }

        if self.clip != prev.clip {
            widgets::TextInput::set_clip(&mut element, self.clip);
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
