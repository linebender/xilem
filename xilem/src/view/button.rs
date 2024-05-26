// Copyright 2024 the Xilem Authors
// SPDX-License-Identifier: Apache-2.0

use crate::{core::View, Pod};
use masonry::{
    widget::{self, WidgetMut},
    ArcStr, WidgetPod,
};

use crate::{MessageResult, ViewCtx, ViewId};

pub fn button<F, State, Action>(label: impl Into<ArcStr>, callback: F) -> Button<F>
where
    F: Fn(&mut State) -> Action + Send + 'static,
{
    Button {
        label: label.into(),
        callback,
    }
}

pub struct Button<F> {
    label: ArcStr,
    callback: F,
}

impl<F, State, Action> View<State, Action, ViewCtx> for Button<F>
where
    F: Fn(&mut State) -> Action + Send + Sync + 'static,
{
    type Element = Pod<widget::Button>;
    type ViewState = ();

    fn build(&self, ctx: &mut ViewCtx) -> (Self::Element, Self::ViewState) {
        ctx.with_leaf_action_widget(|_| {
            Pod::from(WidgetPod::new(widget::Button::new(self.label.clone())))
        })
    }

    fn rebuild(
        &self,
        prev: &Self,
        _: &mut Self::ViewState,
        ctx: &mut ViewCtx,
        mut element: WidgetMut<widget::Button>,
    ) {
        if prev.label != self.label {
            element.set_text(self.label.clone());
            ctx.mark_changed();
        }
    }

    fn teardown(
        &self,
        _: &mut Self::ViewState,
        ctx: &mut ViewCtx,
        element: <Self::Element as xilem_core::ViewElement>::Mut<'_>,
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
                if let masonry::Action::ButtonPressed = *action {
                    MessageResult::Action((self.callback)(app_state))
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
