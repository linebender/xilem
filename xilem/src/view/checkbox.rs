// Copyright 2024 the Xilem Authors
// SPDX-License-Identifier: Apache-2.0

use masonry::core::ArcStr;
use masonry::widgets;

use crate::core::{DynMessage, Mut, ViewMarker};
use crate::{MessageResult, Pod, View, ViewCtx, ViewId};

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
}

impl<F> ViewMarker for Checkbox<F> {}
impl<F, State, Action> View<State, Action, ViewCtx> for Checkbox<F>
where
    F: Fn(&mut State, bool) -> Action + Send + Sync + 'static,
{
    type Element = Pod<widgets::Checkbox>;
    type ViewState = ();

    fn build(&self, ctx: &mut ViewCtx) -> (Self::Element, Self::ViewState) {
        ctx.with_leaf_action_widget(|ctx| {
            ctx.new_pod(widgets::Checkbox::new(self.checked, self.label.clone()))
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
            widgets::Checkbox::set_text(&mut element, self.label.clone());
        }
        if prev.checked != self.checked {
            widgets::Checkbox::set_checked(&mut element, self.checked);
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
        match message.downcast::<masonry::core::Action>() {
            Ok(action) => {
                if let masonry::core::Action::CheckboxToggled(checked) = *action {
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
