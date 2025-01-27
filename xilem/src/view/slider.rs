// Copyright 2024 the Xilem Authors
// SPDX-License-Identifier: Apache-2.0

use core::ops::Range;

use masonry::widgets::{Axis, Slider as MasonrySlider};
use vello::{kurbo::RoundedRectRadii, peniko::Color};
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
    Slider {
        min: range.start,
        max: range.end,
        value: value.clamp(range.start, range.end),
        on_change,
        on_editing_changed: None,
        color: None,
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
    color: Option<Color>,
    track_color: Option<Color>,
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
    pub fn with_color(mut self, color: impl Into<Color>) -> Self {
        self.color = Some(color.into());
        self
    }

    #[must_use]
    /// Set the slider's track color.
    pub fn with_track_color(mut self, track_color: impl Into<Color>) -> Self {
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

macro_rules! define_slider_properties {
    ($($field:ident: $ty:ty => ($with_method:ident, $set_method:ident)),+ $(,)?) => {
        macro_rules! build_slider {
            ($slider:expr, $config:expr) => {{
                let mut s = $slider;
                $(
                    if let Some(v) = $config.$field {
                        s = s.$with_method(v);
                    }
                )*
                s
            }};
        }

        macro_rules! update_slider {
            ($widget:expr, $current:expr, $prev:expr) => {
                $(
                    if $current.$field != $prev.$field {
                        if let Some(v) = $current.$field {
                            MasonrySlider::$set_method($widget, v);
                        }
                    }
                )*
            };
        }
    };
}

// 定义所有需要处理的属性
define_slider_properties! {
    color: Color => (with_color, set_color),
    track_color: Color => (with_track_color, set_track_color),
    step: f64 => (with_step, set_step),
    thumb_radii: RoundedRectRadii => (with_thumb_radii, set_thumb_radii),
    track_radii: RoundedRectRadii => (with_track_radii, set_track_radii),
    hover_glow_color: Color => (with_hover_glow_color, set_hover_glow_color),
    hover_glow_blur_radius: f64 => (with_hover_glow_blur_radius, set_hover_glow_blur_radius),
    hover_glow_spread_radius: f64 => (with_hover_glow_spread_radius, set_hover_glow_spread_radius),
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
            let slider = build_slider!(
                MasonrySlider::new(self.axis, self.min, self.max, self.value),
                self
            );
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

        update_slider!(&mut widget, self, prev);
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
