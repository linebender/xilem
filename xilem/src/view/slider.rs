// Copyright 2024 the Xilem Authors
// SPDX-License-Identifier: Apache-2.0

use core::ops::Range;

use masonry::widget::{self, Axis, Padding, Slider as MasonrySlider};
use vello::kurbo::RoundedRectRadii;
use xilem_core::{DynMessage, MessageResult, Mut, View, ViewId, ViewMarker};

use crate::{Pod, ViewCtx};

use super::Label;

type OnChange<State, Action> = Box<dyn Fn(&mut State, f64) -> Action + Send + Sync + 'static>;
type OnEditingChanged<State, Action> =
    Box<dyn Fn(&mut State, bool) -> Action + Sync + Send + 'static>;

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
pub fn slider<State, Action>(
    range: Range<f64>,
    value: f64,
    on_change: impl Fn(&mut State, f64) -> Action + Send + Sync + 'static,
) -> Slider<State, Action> {
    Slider {
        min: range.start,
        max: range.end,
        value: value.clamp(range.start, range.end),
        on_change: Box::new(on_change),
        on_editing_changed: None,
        color: None,
        track_color: None,
        step: None,
        axis: Axis::Horizontal,
        label: None,
        min_label: None,
        max_label: None,
        label_alignment: None,
        label_padding: None,
        thumb_radii: None,
        track_radii: None,
        hover_glow_color: None,
        hover_glow_blur_radius: None,
        hover_glow_spread_radius: None,
    }
}

/// A slider view that allows selecting a value within a range.
pub struct Slider<State, Action> {
    min: f64,
    max: f64,
    value: f64,
    on_change: OnChange<State, Action>,
    on_editing_changed: Option<OnEditingChanged<State, Action>>,
    color: Option<masonry::Color>,
    track_color: Option<masonry::Color>,
    step: Option<f64>,
    axis: Axis,
    label: Option<Label>,
    min_label: Option<Label>,
    max_label: Option<Label>,
    label_alignment: Option<widget::Alignment>,
    label_padding: Option<Padding>,
    thumb_radii: Option<RoundedRectRadii>,
    track_radii: Option<RoundedRectRadii>,
    hover_glow_color: Option<masonry::Color>,
    hover_glow_blur_radius: Option<f64>,
    hover_glow_spread_radius: Option<f64>,
}

impl<State, Action> ViewMarker for Slider<State, Action> {}

impl<State, Action> Slider<State, Action> {
    /// Set the slider's thumb color.
    pub fn with_color(mut self, color: impl Into<masonry::Color>) -> Self {
        self.color = Some(color.into());
        self
    }

    /// Set the slider's track color.
    pub fn with_track_color(mut self, track_color: impl Into<masonry::Color>) -> Self {
        self.track_color = Some(track_color.into());
        self
    }

    /// Set the slider's step amount.
    pub fn with_step(mut self, step: f64) -> Self {
        self.step = Some(step);
        self
    }

    /// Set the slider's labels.
    pub fn with_labels(mut self, label: Label, min_label: Label, max_label: Label) -> Self {
        self.label = Some(label);
        self.min_label = Some(min_label);
        self.max_label = Some(max_label);
        self
    }

    /// Set the slider's label alignment.
    pub fn with_label_alignment(mut self, alignment: widget::Alignment) -> Self {
        self.label_alignment = Some(alignment);
        self
    }

    /// Set the slider's label padding.
    pub fn with_label_padding(mut self, padding: impl Into<Padding>) -> Self {
        self.label_padding = Some(padding.into());
        self
    }

    /// Set the slider's thumb radii.
    pub fn with_thumb_radii(mut self, radii: impl Into<RoundedRectRadii>) -> Self {
        self.thumb_radii = Some(radii.into());
        self
    }

    /// Set the slider's track radii.
    pub fn with_track_radii(mut self, radii: impl Into<RoundedRectRadii>) -> Self {
        self.track_radii = Some(radii.into());
        self
    }

    /// Set the slider's hover glow color.
    pub fn with_hover_glow_color(mut self, color: impl Into<masonry::Color>) -> Self {
        self.hover_glow_color = Some(color.into());
        self
    }

    /// Set the slider's hover glow blur radius.
    pub fn with_hover_glow_blur_radius(mut self, blur_radius: f64) -> Self {
        self.hover_glow_blur_radius = Some(blur_radius);
        self
    }

    /// Set the slider's hover glow spread radius.
    pub fn with_hover_glow_spread_radius(mut self, spread_radius: f64) -> Self {
        self.hover_glow_spread_radius = Some(spread_radius);
        self
    }

    /// Set the slider's thumb color.
    pub fn vertical(mut self) -> Self {
        self.axis = Axis::Vertical;
        self
    }

    pub fn on_editing_changed<F: Fn(&mut State, bool) -> Action + Send + Sync + 'static>(
        mut self,
        on_editing_changed: F,
    ) -> Self {
        self.on_editing_changed = Some(Box::new(on_editing_changed));
        self
    }
}

impl<F, G> Slider<F, G> {}

impl<State, Action> View<State, Action, ViewCtx> for Slider<State, Action>
where
    Action: 'static,
    State: 'static,
{
    type Element = Pod<MasonrySlider>;
    type ViewState = ();

    fn build(&self, ctx: &mut ViewCtx) -> (Pod<MasonrySlider>, ()) {
        let mut slider = MasonrySlider::new(self.axis, self.min, self.max, self.value);
        if let Some(color) = self.color {
            slider = slider.with_color(color);
        }
        if let Some(track_color) = self.track_color {
            slider = slider.with_track_color(track_color);
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
        ctx.with_leaf_action_widget(|ctx| ctx.new_pod(slider))
    }

    fn rebuild(
        &self,
        prev: &Self,
        state: &mut Self::ViewState,
        ctx: &mut ViewCtx,
        mut element: Mut<Self::Element>,
    ) {
        let mut slider = element.reborrow_mut();
        if self.value != prev.value {
            MasonrySlider::set_value(&mut slider, self.value);
        }
        if self.color != prev.color {
            if let Some(color) = self.color {
                MasonrySlider::set_color(&mut slider, color);
            }
        }

        if self.track_color != prev.track_color {
            if let Some(track_color) = self.track_color {
                MasonrySlider::set_track_color(&mut slider, track_color);
            }
        }
        if self.step != prev.step {
            if let Some(step) = self.step {
                MasonrySlider::set_step(&mut slider, step);
            }
        }

        if self.thumb_radii != prev.thumb_radii {
            if let Some(thumb_radii) = self.thumb_radii {
                MasonrySlider::set_thumb_radii(&mut slider, thumb_radii);
            }
        }
        if self.track_radii != prev.track_radii {
            if let Some(track_radii) = self.track_radii {
                MasonrySlider::set_track_radii(&mut slider, track_radii);
            }
        }
        if self.hover_glow_color != prev.hover_glow_color {
            if let Some(hover_glow_color) = self.hover_glow_color {
                MasonrySlider::set_hover_glow_color(&mut slider, hover_glow_color);
            }
        }
        if self.hover_glow_blur_radius != prev.hover_glow_blur_radius {
            if let Some(hover_glow_blur_radius) = self.hover_glow_blur_radius {
                MasonrySlider::set_hover_glow_blur_radius(&mut slider, hover_glow_blur_radius);
            }
        }
        if self.hover_glow_spread_radius != prev.hover_glow_spread_radius {
            if let Some(hover_glow_spread_radius) = self.hover_glow_spread_radius {
                MasonrySlider::set_hover_glow_spread_radius(&mut slider, hover_glow_spread_radius);
            }
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
        match message.downcast::<masonry::Action>() {
            Ok(action) => {
                if let masonry::Action::SliderValueChanged(value) = *action {
                    MessageResult::Action((self.on_change)(app_state, value))
                } else if let masonry::Action::SliderEditingChanged(value) = *action {
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
