// Copyright 2024 Retype15 (https://github.com/Retype15)
// SPDX-License-Identifier: Apache-2.0

use masonry::peniko::Color;
use masonry::widgets;
use xilem_core::{MessageResult, Mut, View, ViewMarker};

use crate::core::MessageContext;
use crate::{Pod, ViewCtx};

/// A view that displays a [`Slider`] widget.
pub struct Slider<F> {
    min: f64,
    max: f64,
    value: f64,
    on_change: F,
    step: Option<f64>,
    disabled: bool,
    track_color: Option<Color>,
    active_track_color: Option<Color>,
    thumb_color: Option<Color>,
    track_thickness: Option<f64>,
    thumb_radius: Option<f64>,
}

/// Creates a slider widget for selecting a value from a range.
pub fn slider<State, Action>(
    min: f64,
    max: f64,
    value: f64,
    on_change: impl Fn(&mut State, f64) -> Action + Send + Sync + 'static,
) -> Slider<impl Fn(&mut State, f64) -> Action + Send + Sync + 'static> {
    Slider {
        min,
        max,
        value,
        on_change,
        step: None,
        disabled: false,
        track_color: None,
        active_track_color: None,
        thumb_color: None,
        track_thickness: None,
        thumb_radius: None,
    }
}

impl<F> Slider<F> {
    /// Sets the stepping interval of the slider.
    pub fn step(mut self, step: f64) -> Self {
        if step > 0.0 {
            self.step = Some(step);
        }
        self
    }
    /// Sets the color of the inactive part of the track.
    pub fn track_color(mut self, color: impl Into<Color>) -> Self {
        self.track_color = Some(color.into());
        self
    }
    /// Sets the color of the active part of the track and the thumb border.
    pub fn active_track_color(mut self, color: impl Into<Color>) -> Self {
        self.active_track_color = Some(color.into());
        self
    }
    /// Sets the main fill color of the thumb.
    pub fn thumb_color(mut self, color: impl Into<Color>) -> Self {
        self.thumb_color = Some(color.into());
        self
    }
    /// Sets the thickness (height) of the track.
    pub fn track_thickness(mut self, thickness: f64) -> Self {
        if thickness > 0.0 {
            self.track_thickness = Some(thickness);
        }
        self
    }
    /// Sets the base radius of the thumb.
    pub fn thumb_radius(mut self, radius: f64) -> Self {
        if radius > 0.0 {
            self.thumb_radius = Some(radius);
        }
        self
    }
    /// Sets whether the slider is disabled.
    pub fn disabled(mut self, disabled: bool) -> Self {
        self.disabled = disabled;
        self
    }
}

impl<F> ViewMarker for Slider<F> {}

impl<State, Action, F> View<State, Action, ViewCtx> for Slider<F>
where
    State: 'static,
    Action: 'static,
    F: Fn(&mut State, f64) -> Action + Send + Sync + 'static,
{
    type Element = Pod<widgets::Slider>;
    type ViewState = ();

    fn build(&self, ctx: &mut ViewCtx, _app_state: &mut State) -> (Self::Element, Self::ViewState) {
        let pod = ctx.with_action_widget(|ctx| {
            let mut widget = widgets::Slider::new(self.min, self.max, self.value);
            if let Some(step) = self.step {
                widget = widget.with_step(step);
            }
            if let Some(color) = self.track_color {
                widget = widget.with_track_color(color);
            }
            if let Some(color) = self.active_track_color {
                widget = widget.with_active_track_color(color);
            }
            if let Some(color) = self.thumb_color {
                widget = widget.with_thumb_color(color);
            }
            if let Some(thickness) = self.track_thickness {
                widget = widget.with_track_thickness(thickness);
            }
            if let Some(radius) = self.thumb_radius {
                widget = widget.with_thumb_radius(radius);
            }
            widget = widget.with_disabled(self.disabled);

            ctx.create_pod(widget)
        });
        (pod, ())
    }

    fn rebuild(
        &self,
        prev: &Self,
        _view_state: &mut Self::ViewState,
        _ctx: &mut ViewCtx,
        mut element: Mut<'_, Self::Element>,
        _app_state: &mut State,
    ) {
        if prev.min != self.min || prev.max != self.max {
            widgets::Slider::set_range(&mut element, self.min, self.max);
        }
        if prev.step != self.step {
            widgets::Slider::set_step(&mut element, self.step);
        }
        if prev.disabled != self.disabled {
            widgets::Slider::set_disabled(&mut element, self.disabled);
        }
        if prev.value != self.value {
            widgets::Slider::set_value(&mut element, self.value);
        }
    }

    fn teardown(
        &self,
        _view_state: &mut Self::ViewState,
        ctx: &mut ViewCtx,
        element: Mut<'_, Self::Element>,
    ) {
        ctx.teardown_leaf(element);
    }

    fn message(
        &self,
        _view_state: &mut Self::ViewState,
        message: &mut MessageContext,
        _element: Mut<'_, Self::Element>,
        app_state: &mut State,
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
