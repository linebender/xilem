// Copyright 2024 the Xilem Authors
// SPDX-License-Identifier: Apache-2.0

use masonry::text::StyleProperty;
use masonry::widget;
pub use masonry::PointerButton;

use crate::core::{DynMessage, Mut, View, ViewMarker};
use crate::view::Label;
use crate::{Affine, MessageResult, Pod, ViewCtx, ViewId};

use super::Transformable;

/// A button which calls `callback` when the primary mouse button (normally left) is pressed.
///
/// # Examples
/// To use button provide it with a button text and a closure.
/// ```ignore
/// use xilem::view::button;
///
/// struct State {
///     int: i32,
/// }
///
/// impl State {
///     fn increase(&mut self) {
///         self.int += 1;
///     }
/// }
///
/// button("Button", |state: &mut State| {
///      state.increase();
/// })
/// ```
///
/// Create a `button` with a custom `label`.
///
/// ```ignore
/// use xilem::view::{button, label};
///
/// struct State {
///     int: i32,
/// }
///
/// impl State {
///     fn increase(&mut self) {
///         self.int += 1;
///     }
/// }
///
/// let label = label("Button").weight(FontWeight::BOLD);
///
/// button(label, |state: &mut State| {
///     state.increase();
/// })
/// ```
pub fn button<State, Action>(
    label: impl Into<Label>,
    callback: impl Fn(&mut State) -> Action + Send + 'static,
) -> Button<impl for<'a> Fn(&'a mut State, PointerButton) -> MessageResult<Action> + Send + 'static>
{
    Button {
        label: label.into(),
        transform: Affine::IDENTITY,
        callback: move |state: &mut State, button| match button {
            PointerButton::Primary => MessageResult::Action(callback(state)),
            _ => MessageResult::Nop,
        },
    }
}

/// A button which calls `callback` when pressed.
pub fn button_any_pointer<State, Action>(
    label: impl Into<Label>,
    callback: impl Fn(&mut State, PointerButton) -> Action + Send + 'static,
) -> Button<impl for<'a> Fn(&'a mut State, PointerButton) -> MessageResult<Action> + Send + 'static>
{
    Button {
        label: label.into(),
        transform: Affine::IDENTITY,
        callback: move |state: &mut State, button| MessageResult::Action(callback(state, button)),
    }
}

/// The [`View`] created by [`button`] from a `label` and a callback.
///
/// See `button` documentation for more context.
#[must_use = "View values do nothing unless provided to Xilem."]
pub struct Button<F> {
    label: Label,
    transform: Affine,
    callback: F,
}

impl<F> Transformable for Button<F> {
    fn transform_mut(&mut self) -> &mut Affine {
        &mut self.transform
    }
}

impl<F> ViewMarker for Button<F> {}
impl<F, State, Action> View<State, Action, ViewCtx> for Button<F>
where
    F: Fn(&mut State, PointerButton) -> MessageResult<Action> + Send + Sync + 'static,
{
    type Element = Pod<widget::Button>;
    type ViewState = ();

    fn build(&self, ctx: &mut ViewCtx) -> (Self::Element, Self::ViewState) {
        ctx.with_leaf_action_widget(|ctx| {
            ctx.new_pod_with_transform(
                widget::Button::from_label(
                    // TODO: Use `Label::build` here - currently impossible because `Pod` uses `WidgetPod` internally
                    widget::Label::new(self.label.label.clone())
                        .with_brush(self.label.text_brush.clone())
                        .with_alignment(self.label.alignment)
                        .with_style(StyleProperty::FontSize(self.label.text_size))
                        .with_style(StyleProperty::FontWeight(self.label.weight))
                        .with_style(StyleProperty::FontStack(self.label.font.clone())),
                ),
                self.transform,
            )
        })
    }

    fn rebuild(
        &self,
        prev: &Self,
        state: &mut Self::ViewState,
        ctx: &mut ViewCtx,
        mut element: Mut<Self::Element>,
    ) {
        if prev.transform != self.transform {
            element.set_transform(self.transform);
        }

        <Label as View<State, Action, ViewCtx>>::rebuild(
            &self.label,
            &prev.label,
            state,
            ctx,
            widget::Button::label_mut(&mut element),
        );
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
            "id path should be empty in Button::message"
        );
        match message.downcast::<masonry::Action>() {
            Ok(action) => {
                if let masonry::Action::ButtonPressed(button) = *action {
                    (self.callback)(app_state, button)
                } else {
                    tracing::error!("Wrong action type in Button::message: {action:?}");
                    MessageResult::Stale(action)
                }
            }
            Err(message) => {
                tracing::error!("Wrong message type in Button::message: {message:?}");
                MessageResult::Stale(message)
            }
        }
    }
}
