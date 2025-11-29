// Copyright 2025 the Xilem Authors
// SPDX-License-Identifier: Apache-2.0

use std::marker::PhantomData;

use masonry::widgets;

use crate::core::{Arg, MessageCtx, MessageResult, Mut, View, ViewArgument, ViewMarker};
use crate::{Pod, ViewCtx, WidgetView};

/// A view that displays a [`Slider`] widget.
pub struct Slider<State, Action, F> {
    min: f64,
    max: f64,
    value: f64,
    on_change: F,
    step: Option<f64>,
    disabled: bool,
    phantom: PhantomData<fn(State) -> Action>,
}

/// Creates a slider widget for selecting a value from a range.
pub fn slider<
    State: ViewArgument,
    Action,
    F: Fn(Arg<'_, State>, f64) -> Action + Send + Sync + 'static,
>(
    min: f64,
    max: f64,
    value: f64,
    on_change: F,
) -> Slider<State, Action, F>
where
    Slider<State, Action, F>: WidgetView<State, Action>,
{
    Slider {
        min,
        max,
        value,
        on_change,
        step: None,
        disabled: false,
        phantom: PhantomData,
    }
}

impl<State, Action, F> Slider<State, Action, F> {
    /// Sets the stepping interval of the slider.
    pub fn step(mut self, step: f64) -> Self {
        if step > 0.0 {
            self.step = Some(step);
        }
        self
    }
    /// Sets whether the slider is disabled.
    pub fn disabled(mut self, disabled: bool) -> Self {
        self.disabled = disabled;
        self
    }
}

impl<State, Action, F> ViewMarker for Slider<State, Action, F> {}
impl<F, State, Action> View<State, Action, ViewCtx> for Slider<State, Action, F>
where
    State: ViewArgument,
    Action: 'static,
    F: Fn(Arg<'_, State>, f64) -> Action + Send + Sync + 'static,
{
    type Element = Pod<widgets::Slider>;
    type ViewState = ();

    fn build(&self, ctx: &mut ViewCtx, _: Arg<'_, State>) -> (Self::Element, Self::ViewState) {
        (
            ctx.with_action_widget(|ctx| {
                let mut widget = widgets::Slider::new(self.min, self.max, self.value);
                if let Some(step) = self.step {
                    widget = widget.with_step(step);
                }
                let mut pod = ctx.create_pod(widget);
                pod.new_widget.options.disabled = self.disabled;
                pod
            }),
            (),
        )
    }

    fn rebuild(
        &self,
        prev: &Self,
        (): &mut Self::ViewState,
        _: &mut ViewCtx,
        mut element: Mut<'_, Self::Element>,
        _: Arg<'_, State>,
    ) {
        if prev.disabled != self.disabled {
            element.ctx.set_disabled(self.disabled);
        }
        if prev.value != self.value {
            widgets::Slider::set_value(&mut element, self.value);
        }
        if prev.min != self.min || prev.max != self.max {
            widgets::Slider::set_range(&mut element, self.min, self.max);
        }
        if prev.step != self.step {
            widgets::Slider::set_step(&mut element, self.step);
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
        message: &mut MessageCtx,
        _: Mut<'_, Self::Element>,
        app_state: Arg<'_, State>,
    ) -> MessageResult<Action> {
        if message.take_first().is_some() {
            tracing::warn!("Got unexpected id path in Slider::message");
            return MessageResult::Stale;
        }
        match message.take_message::<f64>() {
            Some(value) => MessageResult::Action((self.on_change)(app_state, *value)),
            None => {
                tracing::error!("Wrong message type in Slider::message: {message:?}, expected f64");
                MessageResult::Stale
            }
        }
    }
}
