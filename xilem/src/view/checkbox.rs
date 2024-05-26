// Copyright 2024 the Xilem Authors
// SPDX-License-Identifier: Apache-2.0

use masonry::{
    widget::{self, WidgetMut},
    ArcStr, WidgetPod,
};

use crate::{MessageResult, Pod, View, ViewCtx, ViewId};

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
    }
}

pub struct Checkbox<F> {
    label: ArcStr,
    checked: bool,
    callback: F,
}

impl<F, State, Action> View<State, Action, ViewCtx> for Checkbox<F>
where
    F: Fn(&mut State, bool) -> Action + Send + Sync + 'static,
{
    type Element = Pod<widget::Checkbox>;
    type ViewState = ();

    fn build(&self, ctx: &mut ViewCtx) -> (Self::Element, Self::ViewState) {
        ctx.with_leaf_action_widget(|_| {
            WidgetPod::new(masonry::widget::Checkbox::new(
                self.checked,
                self.label.clone(),
            ))
            .into()
        })
    }

    fn rebuild(
        &self,
        prev: &Self,
        (): &mut Self::ViewState,
        ctx: &mut ViewCtx,
        mut element: WidgetMut<'_, widget::Checkbox>,
    ) {
        if prev.label != self.label {
            element.set_text(self.label.clone());
            ctx.mark_changed();
        }
        if prev.checked != self.checked {
            element.set_checked(self.checked);
            ctx.mark_changed();
        }
    }

    fn teardown(
        &self,
        (): &mut Self::ViewState,
        ctx: &mut ViewCtx,
        element: WidgetMut<'_, widget::Checkbox>,
    ) {
        ctx.teardown_leaf(element);
    }

    fn message(
        &self,
        (): &mut Self::ViewState,
        id_path: &[ViewId],
        message: xilem_core::DynMessage,
        app_state: &mut State,
    ) -> MessageResult<Action> {
        debug_assert!(
            id_path.is_empty(),
            "id path should be empty in Checkbox::message"
        );
        match message.downcast::<masonry::Action>() {
            Ok(action) => {
                if let masonry::Action::CheckboxChecked(checked) = *action {
                    MessageResult::Action((self.callback)(app_state, checked))
                } else {
                    tracing::error!("Wrong action type in Checkbox::message: {action:?}");
                    MessageResult::Stale(action)
                }
            }
            Err(message) => {
                tracing::error!("Wrong message type in Checkbox::message");
                MessageResult::Stale(message)
            }
        }
    }
}
