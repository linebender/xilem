// Copyright 2024 the Xilem Authors
// SPDX-License-Identifier: Apache-2.0

use crate::{core::View, Pod};
use masonry::{widget, ArcStr};
use xilem_core::{Mut, ViewMarker};

pub use masonry::PointerButton;

use crate::{MessageResult, ViewCtx, ViewId};

/// A button which calls `callback` when the primary mouse button (normally left) is pressed.
pub fn button<State, Action>(
    label: impl Into<ArcStr>,
    callback: impl Fn(&mut State) -> Action + Send + 'static,
) -> Button<impl for<'a> Fn(&'a mut State, PointerButton) -> MessageResult<Action> + Send + 'static>
{
    Button {
        label: label.into(),
        callback: move |state: &mut State, button| match button {
            PointerButton::Primary => MessageResult::Action(callback(state)),
            _ => MessageResult::Nop,
        },
    }
}

/// A button which calls `callback` when pressed.
pub fn button_any_pointer<State, Action>(
    label: impl Into<ArcStr>,
    callback: impl Fn(&mut State, PointerButton) -> Action + Send + 'static,
) -> Button<impl for<'a> Fn(&'a mut State, PointerButton) -> MessageResult<Action> + Send + 'static>
{
    Button {
        label: label.into(),
        callback: move |state: &mut State, button| MessageResult::Action(callback(state, button)),
    }
}

pub struct Button<F> {
    label: ArcStr,
    callback: F,
}

impl<F> ViewMarker for Button<F> {}
impl<F, State, Action> View<State, Action, ViewCtx> for Button<F>
where
    F: Fn(&mut State, PointerButton) -> MessageResult<Action> + Send + Sync + 'static,
{
    type Element = Pod<widget::Button>;
    type ViewState = ();

    fn build(&self, ctx: &mut ViewCtx) -> (Self::Element, Self::ViewState) {
        ctx.with_leaf_action_widget(|ctx| ctx.new_pod(widget::Button::new(self.label.clone())))
    }

    fn rebuild<'el>(
        &self,
        prev: &Self,
        _: &mut Self::ViewState,
        ctx: &mut ViewCtx,
        mut element: Mut<'el, Self::Element>,
    ) -> Mut<'el, Self::Element> {
        if prev.label != self.label {
            element.set_text(self.label.clone());
            ctx.mark_changed();
        }
        element
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
        id_path: &[ViewId],
        message: xilem_core::DynMessage,
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
