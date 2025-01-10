// Copyright 2024 the Xilem Authors
// SPDX-License-Identifier: Apache-2.0

use masonry::widget::Slider as MasonrySlider;
use xilem_core::{DynMessage, MessageResult, Mut, View, ViewCtx, ViewId, ViewMarker};

use crate::{Pod, WidgetView};

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
pub fn slider<State, Action>(min: f64, max: f64, value: f64) -> Slider<State, Action> {
    Slider {
        min,
        max,
        value: value.clamp(min, max),
        on_change: None,
        color: None,
    }
}

/// A slider view that allows selecting a value within a range.
pub struct Slider<State, Action> {
    min: f64,
    max: f64,
    value: f64,
    on_change: Option<Box<dyn FnMut(f64) -> Action + Send + Sync>>,
    color: Option<masonry::Color>,
}

impl<State, Action> ViewMarker for Slider<State, Action> {}

impl<State, Action> Slider<State, Action> {
    /// Set a callback for when the slider value changes.
    pub fn on_change(
        mut self,
        on_change: impl FnMut(f64) -> Action + Send + Sync + 'static,
    ) -> Self {
        self.on_change = Some(Box::new(on_change));
        self
    }

    /// Set the slider's thumb color.
    pub fn with_color(mut self, color: impl Into<masonry::Color>) -> Self {
        self.color = Some(color.into());
        self
    }
}

impl<State, Action> View<State, Action, ViewCtx> for Slider<State, Action> {
    type Element = Pod<MasonrySlider>;
    type ViewState = ();

    fn build(&self, ctx: &mut ViewCtx) -> (Pod<MasonrySlider>, ()) {
        let mut slider = MasonrySlider::new(self.min, self.max, self.value);
        
        if let Some(ref on_change) = self.on_change {
            let on_change = on_change.clone();
            slider = slider.on_change(move |value| {
                let action = (on_change)(value);
                ctx.proxy().send(action).unwrap();
            });
        }

        if let Some(color) = self.color {
            slider = slider.with_color(color);
        }

        (ctx.new_pod(slider), ())
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
        _id_path: &[ViewId],
        message: DynMessage,
        _app_state: &mut State,
    ) -> MessageResult<Action> {
        tracing::error!("Message arrived in Slider::message, but Slider doesn't consume any messages, this is a bug");
        MessageResult::Stale(message)
    }
}

impl<State, Action> WidgetView<State, Action> for Slider<State, Action> {
    type Widget = MasonrySlider;
}
