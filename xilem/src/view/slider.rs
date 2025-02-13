// Copyright 2024 the Xilem Authors
// SPDX-License-Identifier: Apache-2.0

use core::ops::Range;

use masonry::{
    theme,
    widgets::{Axis, Slider as MasonrySlider},
};
use vello::{
    kurbo::RoundedRectRadii,
    peniko::{Brush, Color},
};
use xilem_core::{DynMessage, MessageResult, Mut, View, ViewId, ViewMarker};

use crate::{Pod, ViewCtx};

type OnEditingChanged<State, Action> =
    Box<dyn Fn(&mut State, bool) -> Action + Sync + Send + 'static>;

/// A slider widget for selecting a value within a range.
pub fn slider<F, State, Action>(
    range: Range<f64>,
    value: f64,
    on_change: F,
) -> Slider<F, State, Action>
where
    F: Fn(&mut State, f64) -> Action + Send + 'static,
{
    let (min, max) = (range.start.min(range.end), range.start.max(range.end));
    Slider {
        min,
        max,
        value: value.clamp(min, max),
        on_change,
        on_editing_changed: None,
        thumb_color: None,
        track_color: None,
        step: None,
        axis: Axis::Horizontal,
        thumb_radii: None,
        track_radii: None,
        hover_glow_color: None,
        hover_glow_blur_radius: None,
        hover_glow_spread_radius: None,
    }
}

/// A slider view that allows selecting a value within a range.
pub struct Slider<F, State, Action> {
    min: f64,
    max: f64,
    value: f64,
    on_change: F,
    on_editing_changed: Option<OnEditingChanged<State, Action>>,
    thumb_color: Option<Brush>,
    track_color: Option<Brush>,
    step: Option<f64>,
    axis: Axis,
    thumb_radii: Option<RoundedRectRadii>,
    track_radii: Option<RoundedRectRadii>,
    hover_glow_color: Option<Color>,
    hover_glow_blur_radius: Option<f64>,
    hover_glow_spread_radius: Option<f64>,
}

impl<F, State, Action> ViewMarker for Slider<F, State, Action> {}

impl<F, State, Action> Slider<F, State, Action> {
    #[must_use]
    /// Set the slider's direction (horizontal or vertical).
    pub fn direction(mut self, axis: Axis) -> Self {
        self.axis = axis;
        self
    }

    #[must_use]
    /// Set the slider's thumb color.
    pub fn with_thumb_color(mut self, thumb_color: impl Into<Brush>) -> Self {
        self.thumb_color = Some(thumb_color.into());
        self
    }

    #[must_use]
    /// Set the slider's track color.
    pub fn with_track_color(mut self, track_color: impl Into<Brush>) -> Self {
        self.track_color = Some(track_color.into());
        self
    }

    #[must_use]
    /// Set the slider's step amount.
    pub fn with_step(mut self, step: f64) -> Self {
        self.step = Some(step);
        self
    }

    #[must_use]
    /// Set the slider's thumb radii.
    pub fn with_thumb_radii(mut self, radii: impl Into<RoundedRectRadii>) -> Self {
        self.thumb_radii = Some(radii.into());
        self
    }

    #[must_use]
    /// Set the slider's track radii.
    pub fn with_track_radii(mut self, radii: impl Into<RoundedRectRadii>) -> Self {
        self.track_radii = Some(radii.into());
        self
    }

    #[must_use]
    /// Set the slider's hover glow color.
    pub fn with_hover_glow_color(mut self, color: impl Into<Color>) -> Self {
        self.hover_glow_color = Some(color.into());
        self
    }

    #[must_use]
    /// Set the slider's hover glow blur radius.
    pub fn with_hover_glow_blur_radius(mut self, blur_radius: f64) -> Self {
        self.hover_glow_blur_radius = Some(blur_radius);
        self
    }

    #[must_use]
    /// Set the slider's hover glow spread radius.
    pub fn with_hover_glow_spread_radius(mut self, spread_radius: f64) -> Self {
        self.hover_glow_spread_radius = Some(spread_radius);
        self
    }

    #[must_use]
    /// Set the callback for editing state changes.
    pub fn on_editing_changed(
        mut self,
        on_editing_changed: impl Fn(&mut State, bool) -> Action + Send + Sync + 'static,
    ) -> Self {
        self.on_editing_changed = Some(Box::new(on_editing_changed));
        self
    }
}

impl<F, State, Action> View<State, Action, ViewCtx> for Slider<F, State, Action>
where
    F: Fn(&mut State, f64) -> Action + Send + 'static,
    Action: 'static,
    State: 'static,
{
    type Element = Pod<MasonrySlider>;
    type ViewState = ();

    fn build(&self, ctx: &mut ViewCtx) -> (Pod<MasonrySlider>, ()) {
        ctx.with_leaf_action_widget(|ctx| {
            let mut slider = MasonrySlider::new(self.axis, self.min, self.max, self.value);

            if let Some(thumb_color) = &self.thumb_color {
                slider = slider.with_thumb_color(thumb_color.clone());
            }
            if let Some(track_color) = &self.track_color {
                slider = slider.with_track_color(track_color.clone());
            }
            if let Some(step) = self.step {
                slider = slider.with_step(step);
            }
            if let Some(thumb_radii) = self.thumb_radii {
                slider = slider.with_thumb_radii(thumb_radii);
            }
            if let Some(track_radii) = self.track_radii {
                slider = slider.with_track_radii(track_radii);
            }
            if let Some(hover_glow_color) = self.hover_glow_color {
                slider = slider.with_hover_glow_color(hover_glow_color);
            }
            if let Some(hover_glow_blur_radius) = self.hover_glow_blur_radius {
                slider = slider.with_hover_glow_blur_radius(hover_glow_blur_radius);
            }
            if let Some(hover_glow_spread_radius) = self.hover_glow_spread_radius {
                slider = slider.with_hover_glow_spread_radius(hover_glow_spread_radius);
            }

            ctx.new_pod(slider)
        })
    }

    fn rebuild(
        &self,
        prev: &Self,
        _state: &mut Self::ViewState,
        _ctx: &mut ViewCtx,
        mut element: Mut<Self::Element>,
    ) {
        let mut widget = element.reborrow_mut();

        if self.value != prev.value {
            MasonrySlider::set_value(&mut widget, self.value);
        }

        if self.min != prev.min {
            MasonrySlider::set_min(&mut widget, self.min);
        }

        if self.max != prev.max {
            MasonrySlider::set_max(&mut widget, self.max);
        }

        if self.axis != prev.axis {
            MasonrySlider::set_axis(&mut widget, self.axis);
        }

        if self.thumb_color != prev.thumb_color {
            match &self.thumb_color {
                Some(thumb_color) => {
                    MasonrySlider::set_thumb_color(&mut widget, thumb_color.clone());
                }
                None => MasonrySlider::set_thumb_color(&mut widget, theme::PRIMARY_LIGHT),
            }
        }

        if self.track_color != prev.track_color {
            match &self.track_color {
                Some(track_color) => {
                    MasonrySlider::set_track_color(&mut widget, track_color.clone());
                }
                None => MasonrySlider::set_track_color(&mut widget, theme::PRIMARY_DARK),
            }
        }

        if self.step != prev.step {
            match self.step {
                Some(step) => MasonrySlider::set_step(&mut widget, step),
                None => MasonrySlider::set_step(&mut widget, theme::SLIDER_STEP),
            }
        }

        if self.thumb_radii != prev.thumb_radii {
            match self.thumb_radii {
                Some(thumb_radii) => MasonrySlider::set_thumb_radii(&mut widget, thumb_radii),
                None => MasonrySlider::set_thumb_radii(
                    &mut widget,
                    RoundedRectRadii::from_single_radius(theme::SLIDER_THUMB_RADIUS),
                ),
            }
        }

        if self.track_radii != prev.track_radii {
            match self.track_radii {
                Some(track_radii) => MasonrySlider::set_track_radii(&mut widget, track_radii),
                None => MasonrySlider::set_track_radii(
                    &mut widget,
                    RoundedRectRadii::from_single_radius(theme::SLIDER_TRACK_RADIUS),
                ),
            };
        }

        if self.hover_glow_color != prev.hover_glow_color {
            match self.hover_glow_color {
                Some(hover_glow_color) => {
                    MasonrySlider::set_hover_glow_color(&mut widget, hover_glow_color);
                }

                None => {
                    MasonrySlider::set_hover_glow_color(
                        &mut widget,
                        theme::SLIDER_HOVER_GLOW_COLOR,
                    );
                }
            };
        }

        if self.hover_glow_blur_radius != prev.hover_glow_blur_radius {
            match self.hover_glow_blur_radius {
                Some(hover_glow_blur_radius) => {
                    MasonrySlider::set_hover_glow_blur_radius(&mut widget, hover_glow_blur_radius);
                }
                None => MasonrySlider::set_hover_glow_blur_radius(
                    &mut widget,
                    theme::SLIDER_HOVER_GLOW_BLUR_RADIUS,
                ),
            };
        }

        if self.hover_glow_spread_radius != prev.hover_glow_spread_radius {
            match self.hover_glow_spread_radius {
                Some(hover_glow_spread_radius) => MasonrySlider::set_hover_glow_spread_radius(
                    &mut widget,
                    hover_glow_spread_radius,
                ),
                None => MasonrySlider::set_hover_glow_spread_radius(
                    &mut widget,
                    theme::SLIDER_HOVER_GLOW_SPREAD_RADIUS,
                ),
            };
        }
    }

    fn teardown(
        &self,
        _state: &mut Self::ViewState,
        ctx: &mut ViewCtx,
        element: Mut<Self::Element>,
    ) {
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
        match message.downcast::<masonry::core::Action>() {
            Ok(action) => {
                if let masonry::core::Action::SliderValueChanged(value) = *action {
                    MessageResult::Action((self.on_change)(app_state, value))
                } else if let masonry::core::Action::SliderEditingChanged(value) = *action {
                    if let Some(ref on_editing_changed) = self.on_editing_changed {
                        MessageResult::Action((on_editing_changed)(app_state, value))
                    } else {
                        MessageResult::Stale(action)
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
