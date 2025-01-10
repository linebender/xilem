// Copyright 2024 the Xilem Authors
// SPDX-License-Identifier: Apache-2.0

use masonry::widget::Slider as MasonrySlider;
use xilem_core::{MessageResult, View, ViewCtx};

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
        mut element: xilem_core::Mut<Self::Element>,
        _ctx: &mut ViewCtx,
        _state: &mut Self::ViewState,
    ) -> MessageResult<Action> {
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
        MessageResult::Nop
    }
}

impl<State, Action> WidgetView<State, Action> for Slider<State, Action> {
    type Widget = MasonrySlider;
}
