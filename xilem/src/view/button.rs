// Copyright 2024 the Xilem Authors
// SPDX-License-Identifier: Apache-2.0

use masonry::widget;
pub use masonry::PointerButton;
use xilem_core::ViewPathTracker;

use crate::core::{DynMessage, Mut, View, ViewMarker};
use crate::view::Label;
use crate::{Affine, MessageResult, Pod, ViewCtx, ViewId};

use super::Transformable;

/// A button which calls `callback` when the primary mouse button (normally left) is pressed.
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

#[must_use = "View values do nothing unless provided to Xilem."]
pub struct Button<F> {
    // N.B. This widget is *implemented* to handle any kind of view with an element
    // type of `Label` even though it currently does not do so.
    label: Label,
    transform: Affine,
    callback: F,
}

impl<F> Transformable for Button<F> {
    fn transform_mut(&mut self) -> &mut Affine {
        &mut self.transform
    }
}

const LABEL_VIEW_ID: ViewId = ViewId::new(0);

impl<F> ViewMarker for Button<F> {}
impl<F, State, Action> View<State, Action, ViewCtx> for Button<F>
where
    F: Fn(&mut State, PointerButton) -> MessageResult<Action> + Send + Sync + 'static,
{
    type Element = Pod<widget::Button>;
    type ViewState = ();

    fn build(&self, ctx: &mut ViewCtx) -> (Self::Element, Self::ViewState) {
        let (child, ()) = ctx.with_id(LABEL_VIEW_ID, |ctx| {
            View::<State, Action, _>::build(&self.label, ctx)
        });
        ctx.with_leaf_action_widget(|ctx| {
            ctx.new_pod_with_transform(
                widget::Button::from_label_pod(child.into_widget_pod()),
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

        ctx.with_id(LABEL_VIEW_ID, |ctx| {
            View::<State, Action, _>::rebuild(
                &self.label,
                &prev.label,
                state,
                ctx,
                widget::Button::label_mut(&mut element),
            );
        });
    }

    fn teardown(
        &self,
        _: &mut Self::ViewState,
        ctx: &mut ViewCtx,
        mut element: Mut<Self::Element>,
    ) {
        ctx.with_id(LABEL_VIEW_ID, |ctx| {
            View::<State, Action, _>::teardown(
                &self.label,
                &mut (),
                ctx,
                widget::Button::label_mut(&mut element),
            );
        });
        ctx.teardown_leaf(element);
    }

    fn message(
        &self,
        _: &mut Self::ViewState,
        id_path: &[ViewId],
        message: DynMessage,
        app_state: &mut State,
    ) -> MessageResult<Action> {
        match id_path.split_first() {
            Some((&LABEL_VIEW_ID, rest)) => self.label.message(&mut (), rest, message, app_state),
            None => match message.downcast::<masonry::Action>() {
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
            },
            _ => {
                tracing::warn!("Got unexpected id path in Button::message");
                MessageResult::Stale(message)
            }
        }
    }
}
