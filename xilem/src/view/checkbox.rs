// Copyright 2024 the Xilem Authors
// SPDX-License-Identifier: Apache-2.0

use masonry::{widget::WidgetMut, ArcStr, WidgetPod};

use crate::{MasonryView, MessageResult, ViewCtx, ViewId};

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

impl<F, State, Action> MasonryView<State, Action> for Checkbox<F>
where
    F: Fn(&mut State, bool) -> Action + Send + Sync + 'static,
{
    type Element = masonry::widget::Checkbox;
    type ViewState = ();

    fn build(&self, cx: &mut ViewCtx) -> (WidgetPod<Self::Element>, Self::ViewState) {
        cx.with_leaf_action_widget(|_| {
            WidgetPod::new(masonry::widget::Checkbox::new(
                self.checked,
                self.label.clone(),
            ))
        })
    }

    fn rebuild(
        &self,
        _view_state: &mut Self::ViewState,
        cx: &mut ViewCtx,
        prev: &Self,
        mut element: WidgetMut<Self::Element>,
    ) {
        if prev.label != self.label {
            element.set_text(self.label.clone());
            cx.mark_changed();
        }
        if prev.checked != self.checked {
            element.set_checked(self.checked);
            cx.mark_changed();
        }
    }

    fn message(
        &self,
        _view_state: &mut Self::ViewState,
        id_path: &[ViewId],
        message: Box<dyn std::any::Any>,
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
