// Copyright 2024 the Xilem Authors
// SPDX-License-Identifier: Apache-2.0

use masonry::widget::Slider as MasonrySlider;
use xilem_core::{DynMessage, MessageResult, Mut, View, ViewId, ViewMarker};

use crate::{Pod, ViewCtx, WidgetView};

/// A slider widget for selecting a value within a range.
///
/// # Example
/// ```rust
/// use xilem::view::slider;
/// use xilem::Color;
///
/// slider(0.0, 1.0, 0.5)
///     .on_change(|value| println!("Slider value: {}", value))
///     .with_color(Color::rgb8(100, 150, 200));
/// ```
pub fn slider(min: f64, max: f64, value: f64) -> Slider<impl for<'a> Fn(&'a mut (), f64) -> MessageResult<()> + Send + 'static> {
    Slider {
        min,
        max,
        value: value.clamp(min, max),
        on_change: None,
        color: None,
    }
}

/// A slider view that allows selecting a value within a range.
pub struct Slider<F> {
    min: f64,
    max: f64,
    value: f64,
    on_change: Option<F>,
    color: Option<masonry::Color>,
}

impl<F> ViewMarker for Slider<F> {}

impl<F> Slider<F>
where
    F: for<'a> Fn(&'a mut (), f64) -> MessageResult<()> + Send + Sync + 'static,
{
    /// Set a callback for when the slider value changes.
    pub fn on_change<State, Action>(self, on_change: impl Fn(&mut State, f64) -> Action + Send + Sync + 'static) -> Slider<impl for<'a> Fn(&'a mut State, f64) -> MessageResult<Action> + Send + 'static> {
        Slider {
            min: self.min,
            max: self.max,
            value: self.value,
            on_change: Some(move |state: &mut State, value| MessageResult::Action(on_change(state, value))),
            color: self.color,
        }
    }

    /// Set the slider's thumb color.
    pub fn with_color(mut self, color: impl Into<masonry::Color>) -> Self {
        self.color = Some(color.into());
        self
    }
}

impl<F, State, Action> View<State, Action, ViewCtx> for Slider<F>
where
    F: Fn(&mut State, f64) -> MessageResult<Action> + Send + Sync + 'static,
{
    type Element = Pod<MasonrySlider>;
    type ViewState = ();

    fn build(&self, ctx: &mut ViewCtx) -> (Pod<MasonrySlider>, ()) {
        let slider = MasonrySlider::new(self.min, self.max, self.value);
        if let Some(color) = self.color {
            slider = slider.with_color(color);
        }
        ctx.with_leaf_action_widget(|ctx| ctx.new_pod(slider))
    }

    fn rebuild(
        &self,
        prev: &Self,
        _state: &mut Self::ViewState,
        ctx: &mut ViewCtx,
        mut element: Mut<Self::Element>,
    ) {
        element.with_downcast_mut(|slider| {
            if self.value != prev.value {
                slider.set_value(self.value);
            }
            if self.color != prev.color {
                if let Some(color) = self.color {
                    slider.set_color(color);
                }
            }
        });
    }

    fn teardown(&self, _state: &mut Self::ViewState, ctx: &mut ViewCtx, element: Mut<Self::Element>) {
        ctx.teardown_leaf(element);
    }

    fn message(
        &self,
        _state: &mut Self::ViewState,
        id_path: &[ViewId],
        message: DynMessage,
        app_state: &mut State,
    ) -> MessageResult<Action> {
        debug_assert!(
            id_path.is_empty(),
            "id path should be empty in Slider::message"
        );
        match message.downcast::<masonry::Action>() {
            Ok(action) => {
                if let masonry::Action::SliderValueChanged(value) = *action {
                    if let Some(on_change) = &self.on_change {
                        MessageResult::Action(on_change(app_state, value))
                    } else {
                        MessageResult::Nop
                    }
                } else {
                    tracing::error!("Wrong action type in Slider::message: {action:?}");
                    MessageResult::Stale(action)
                }
            }
            Err(message) => {
                tracing::error!("Wrong message type in Slider::message: {message:?}");
                MessageResult::Stale(message)
            }
        }
    }
}

impl<State, Action, F> WidgetView<State, Action> for Slider<State, Action, F>
where
    F: Fn(&mut State, f64) -> Action + Send + Sync + 'static,
{
    type Widget = MasonrySlider;
}
