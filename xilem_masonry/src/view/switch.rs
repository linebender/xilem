// Copyright 2025 the Xilem Authors
// SPDX-License-Identifier: Apache-2.0

use std::marker::PhantomData;

use masonry::widgets::{self, SwitchToggled};

use crate::core::{MessageCtx, MessageResult, Mut, View, ViewMarker};
use crate::{Pod, ViewCtx};

/// A switch switch element which can be in on and off state.
///
/// # Example
/// ```
/// # use xilem_masonry as xilem;
/// use xilem::view::switch;
/// # use xilem::WidgetView;
/// # use xilem::core::Edit;
///
/// struct State {
///     value: bool,
/// }
///
/// # fn view(app_state: &mut State) -> impl WidgetView<Edit<State>> {
/// switch(app_state.value, |app_state: &mut State, new_state: bool| {
///     app_state.value = new_state;
/// })
/// # }
/// ```
pub fn switch<F, State, Action>(on: bool, callback: F) -> Switch<State, Action, F>
where
    F: Fn(&mut State, bool) -> Action + Send + Sync + 'static,
    State: 'static,
{
    Switch {
        on,
        callback,
        disabled: false,
        phantom: PhantomData,
    }
}

/// The [`View`] created by [`switch`] from a bool value and a callback.
///
/// See `switch` documentation for more context.
#[must_use = "View values do nothing unless provided to Xilem."]
pub struct Switch<State, Action, F> {
    on: bool,
    callback: F,
    disabled: bool,
    phantom: PhantomData<fn(State) -> Action>,
}

impl<State, Action, F> Switch<State, Action, F> {
    /// Set the disabled state of the widget.
    pub fn disabled(mut self, disabled: bool) -> Self {
        self.disabled = disabled;
        self
    }
}

impl<State, Action, F> ViewMarker for Switch<State, Action, F> {}
impl<F, State, Action> View<State, Action, ViewCtx> for Switch<State, Action, F>
where
    State: 'static,
    Action: 'static,
    F: Fn(&mut State, bool) -> Action + Send + Sync + 'static,
{
    type Element = Pod<widgets::Switch>;
    type ViewState = ();

    fn build(&self, ctx: &mut ViewCtx, _: &mut State) -> (Self::Element, Self::ViewState) {
        let element = ctx.with_action_widget(|ctx| {
            let mut pod = ctx.create_pod(widgets::Switch::new(self.on));
            pod.new_widget.options.disabled = self.disabled;
            pod
        });
        (element, ())
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
        if prev.on != self.on {
            widgets::Switch::set_on(&mut element, self.on);
        }
    }

    fn teardown(
        &self,
        (): &mut Self::ViewState,
        ctx: &mut ViewCtx,
        element: Mut<'_, Self::Element>,
    ) {
        ctx.teardown_action_source(element);
    }

    fn message(
        &self,
        (): &mut Self::ViewState,
        message: &mut MessageCtx,
        _element: Mut<'_, Self::Element>,
        app_state: &mut State,
    ) -> MessageResult<Action> {
        debug_assert!(
            message.remaining_path().is_empty(),
            "id path should be empty in Switch::message"
        );
        match message.take_message::<SwitchToggled>() {
            Some(switched) => MessageResult::Action((self.callback)(app_state, switched.0)),
            None => {
                tracing::error!("Wrong message type in Switch::message, got {message:?}.");
                MessageResult::Stale
            }
        }
    }
}
