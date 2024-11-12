// Copyright 2024 the Xilem Authors
// SPDX-License-Identifier: Apache-2.0

use masonry::text::ArcStr;
use masonry::widget;

use crate::core::{DynMessage, Mut, ViewMarker};
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

#[must_use = "View values do nothing unless provided to Xilem."]
pub struct Checkbox<F> {
    label: ArcStr,
    checked: bool,
    callback: F,
}

impl<F> ViewMarker for Checkbox<F> {}
impl<F, State, Action> View<State, Action, ViewCtx> for Checkbox<F>
where
    F: Fn(&mut State, bool) -> Action + Send + Sync + 'static,
{
    type Element = Pod<widget::Checkbox>;
    type ViewState = ();

    fn build(&self, ctx: &mut ViewCtx) -> (Self::Element, Self::ViewState) {
        ctx.with_leaf_action_widget(|ctx| {
            ctx.new_pod(widget::Checkbox::new(self.checked, self.label.clone()))
        })
    }

    fn rebuild(
        &self,
        prev: &Self,
        (): &mut Self::ViewState,
        _ctx: &mut ViewCtx,
        mut element: Mut<Self::Element>,
    ) {
        if prev.label != self.label {
            widget::Checkbox::set_text(&mut element, self.label.clone());
        }
        if prev.checked != self.checked {
            widget::Checkbox::set_checked(&mut element, self.checked);
        }
    }

    fn teardown(&self, (): &mut Self::ViewState, ctx: &mut ViewCtx, element: Mut<Self::Element>) {
        ctx.teardown_leaf(element);
    }

    fn message(
        &self,
        (): &mut Self::ViewState,
        id_path: &[ViewId],
        message: DynMessage,
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
