// Copyright 2024 the Xilem Authors
// SPDX-License-Identifier: Apache-2.0

use crate::core::{MessageContext, Mut, ViewMarker};
use crate::{MessageResult, Pod, View, ViewCtx};

use masonry::core::{ArcStr, NewWidget};
use masonry::parley::StyleProperty;
use masonry::parley::style::{FontStack, FontWeight};
use masonry::widgets::{self, CheckboxToggled};

/// An element which can be in checked and unchecked state.
///
/// # Example
/// ```ignore
/// use xilem::view::checkbox;
///
/// struct State {
///     value: bool,
/// }
///
/// // ...
///
/// let new_state = false;
///
/// checkbox("A simple checkbox", app_state.value, |app_state: &mut State, new_state: bool| {
/// *app_state.value = new_state;
/// })
/// ```
pub fn checkbox<F, State, Action>(
    label: impl Into<ArcStr>,
    checked: bool,
    callback: F,
) -> Checkbox<F>
where
    F: Fn(&mut State, bool) -> Action + Send + 'static,
{
    Checkbox {
        label: label.into(),
        callback,
        checked,
        text_size: masonry::theme::TEXT_SIZE_NORMAL,
        weight: FontWeight::NORMAL,
        font: FontStack::List(std::borrow::Cow::Borrowed(&[])),
        disabled: false,
    }
}

/// The [`View`] created by [`checkbox`] from a `label`, a bool value and a callback.
///
/// See `checkbox` documentation for more context.
#[must_use = "View values do nothing unless provided to Xilem."]
pub struct Checkbox<F> {
    label: ArcStr,
    checked: bool,
    callback: F,
    text_size: f32,
    weight: FontWeight,
    font: FontStack<'static>,
    disabled: bool,
}

impl<F> Checkbox<F> {
    /// Sets text size of the checkbox label.
    #[doc(alias = "font_size")]
    pub fn text_size(mut self, text_size: f32) -> Self {
        self.text_size = text_size;
        self
    }

    /// Sets font weight of the checkbox label.
    pub fn weight(mut self, weight: FontWeight) -> Self {
        self.weight = weight;
        self
    }

    /// Set the [font stack](FontStack) the checkbox label will use.
    ///
    /// A font stack allows for providing fallbacks. If there is no matching font
    /// for a character, a system font will be used (if the system fonts are enabled).
    pub fn font(mut self, font: impl Into<FontStack<'static>>) -> Self {
        self.font = font.into();
        self
    }

    /// Set the disabled state of the widget.
    pub fn disabled(mut self, disabled: bool) -> Self {
        self.disabled = disabled;
        self
    }
}

impl<F> ViewMarker for Checkbox<F> {}
impl<F, State, Action> View<State, Action, ViewCtx> for Checkbox<F>
where
    F: Fn(&mut State, bool) -> Action + Send + Sync + 'static,
{
    type Element = Pod<widgets::Checkbox>;
    type ViewState = ();

    fn build(&self, ctx: &mut ViewCtx, _: &mut State) -> (Self::Element, Self::ViewState) {
        let label = widgets::Label::new(self.label.clone())
            .with_style(StyleProperty::FontSize(self.text_size))
            .with_style(StyleProperty::FontWeight(self.weight))
            .with_style(StyleProperty::FontStack(self.font.clone()));

        ctx.with_leaf_action_widget(|ctx| {
            let mut pod = ctx.create_pod(widgets::Checkbox::from_label(
                self.checked,
                NewWidget::new(label),
            ));
            pod.new_widget.options.disabled = self.disabled;
            pod
        })
    }

    fn rebuild(
        &self,
        prev: &Self,
        (): &mut Self::ViewState,
        _ctx: &mut ViewCtx,
        mut element: Mut<'_, Self::Element>,
        _: &mut State,
    ) {
        if prev.disabled != self.disabled {
            element.ctx.set_disabled(self.disabled);
        }
        if prev.label != self.label {
            widgets::Checkbox::set_text(&mut element, self.label.clone());
        }
        if prev.checked != self.checked {
            widgets::Checkbox::set_checked(&mut element, self.checked);
        }

        let mut label = widgets::Checkbox::label_mut(&mut element);
        if prev.text_size != self.text_size {
            widgets::Label::insert_style(&mut label, StyleProperty::FontSize(self.text_size));
        }
        if prev.weight != self.weight {
            widgets::Label::insert_style(&mut label, StyleProperty::FontWeight(self.weight));
        }
        if prev.font != self.font {
            widgets::Label::insert_style(&mut label, StyleProperty::FontStack(self.font.clone()));
        }
    }

    fn teardown(
        &self,
        (): &mut Self::ViewState,
        ctx: &mut ViewCtx,
        element: Mut<'_, Self::Element>,
    ) {
        ctx.teardown_leaf(element);
    }

    fn message(
        &self,
        (): &mut Self::ViewState,
        message: &mut MessageContext,
        _element: Mut<'_, Self::Element>,
        app_state: &mut State,
    ) -> MessageResult<Action> {
        debug_assert!(
            message.remaining_path().is_empty(),
            "id path should be empty in Checkbox::message"
        );
        match message.take_message::<CheckboxToggled>() {
            Some(checked) => MessageResult::Action((self.callback)(app_state, checked.0)),
            None => {
                tracing::error!("Wrong message type in Checkbox::message, got {message:?}.");
                MessageResult::Stale
            }
        }
    }
}
