// Copyright 2019 the Xilem Authors and the Druid Authors
// SPDX-License-Identifier: Apache-2.0

use std::cmp::Ordering;

use accesskit::{Node, Role};
use parley::style::FontWeight;
use tracing::{Span, trace_span};
use vello::Scene;

use crate::core::{
    AccessCtx, ArcStr, ChildrenIds, LayoutCtx, MeasureCtx, NewWidget, NoAction, PaintCtx,
    PropertiesMut, PropertiesRef, RegisterCtx, StyleProperty, Update, UpdateCtx, Widget, WidgetId,
    WidgetMut, WidgetPod,
};
use crate::kurbo::{Axis, Point, Size};
use crate::layout::LenReq;
use crate::widgets::Label;

/// An `f32` value which can move towards a target value at a linear rate over time.
#[derive(Clone, Debug)]
pub struct AnimatedF32 {
    /// The value which self will eventually reach.
    target: f32,
    /// The current value
    value: f32,
    // TODO: Provide different easing functions, instead of just linear
    /// The change in value every millisecond, which will not change over the lifetime of the value.
    rate_per_millisecond: f32,
}

impl AnimatedF32 {
    /// Creates a value which is not changing.
    pub fn stable(value: f32) -> Self {
        assert!(value.is_finite(), "invalid animated value");
        Self {
            target: value,
            value,
            rate_per_millisecond: 0.,
        }
    }

    /// Moves this value to the `target` over `over_millis` milliseconds.
    /// Might change the current value, if `over_millis` is zero.
    ///
    /// `over_millis` should be non-negative.
    ///
    /// # Panics
    ///
    /// If `target` is not a finite value.
    pub fn move_to(&mut self, target: f32, over_millis: f32) {
        assert!(target.is_finite(), "invalid target value");
        assert!(over_millis.is_finite(), "invalid delay value");
        self.target = target;
        match over_millis.partial_cmp(&0.) {
            Some(Ordering::Equal) => self.value = target,
            Some(Ordering::Less) => {
                tracing::warn!("move_to: provided negative time step {over_millis}");
                self.value = target;
            }
            Some(Ordering::Greater) => {
                // Since over_millis is positive, we know that this vector is in the direction of the `target`.
                self.rate_per_millisecond = (self.target - self.value) / over_millis;
                debug_assert!(
                    self.rate_per_millisecond.is_finite(),
                    "Calculated invalid rate despite valid inputs. Current value is {}",
                    self.value
                );
            }
            None => panic!("Provided invalid time step {over_millis}"),
        }
    }

    /// Advances this animation by `by_millis` milliseconds.
    ///
    /// Returns the status of the animation after this advancement.
    pub fn advance(&mut self, by_millis: f32) -> AnimationStatus {
        assert!(by_millis.is_finite(), "invalid timestep value");

        let original_side = self
            .value
            .partial_cmp(&self.target)
            .expect("Target and value are not NaN.");

        self.value += self.rate_per_millisecond * by_millis;
        let other_side = self
            .value
            .partial_cmp(&self.target)
            .expect("Target and value are not NaN.");

        if other_side.is_eq() || original_side != other_side {
            self.value = self.target;
            self.rate_per_millisecond = 0.;
            AnimationStatus::Completed
        } else {
            AnimationStatus::Ongoing
        }
    }
}

/// The status an animation can be in.
///
/// Generally returned when an animation is advanced, to determine whether.
#[derive(Copy, Clone, Debug, PartialEq, Eq)]
pub enum AnimationStatus {
    /// The animation has finished.
    Completed,
    /// The animation is still running
    Ongoing,
}

impl AnimationStatus {
    /// Return true if animation has finished.
    pub fn is_completed(self) -> bool {
        matches!(self, Self::Completed)
    }
}

/// A widget displaying non-editable text, with a variable [weight](parley::style::FontWeight).
///
/// Ensure that `VariableLabel` has [`Dimensions`] set via props
/// either to [`Dimensions::fixed`] or [`Dimensions::MAX`].
/// Fixed dimensions resolve early and are explicit in intent.
/// Max preferred size of `VariableLabel` means that the question of size
/// will get passed through to its inner label, and doesn't mean that it will
/// necessarily map to the max preferred size of the label.
///
/// [`Dimensions`]: crate::properties::Dimensions
/// [`Dimensions::fixed`]: crate::properties::Dimensions::fixed
/// [`Dimensions::MAX`]: crate::properties::Dimensions::MAX
pub struct VariableLabel {
    label: WidgetPod<Label>,
    weight: AnimatedF32,
}

// --- MARK: BUILDERS
impl VariableLabel {
    /// Creates a new variable label from the given text.
    pub fn new(text: impl Into<ArcStr>) -> Self {
        Self::from_label(NewWidget::new(Label::new(text)))
    }

    /// Creates a new variable label from the given label.
    ///
    /// Uses the label's text and style values.
    pub fn from_label(label: NewWidget<Label>) -> Self {
        Self {
            label: label.to_pod(),
            weight: AnimatedF32::stable(FontWeight::NORMAL.value()),
        }
    }

    /// Sets the initial font weight for this text.
    pub fn with_initial_weight(mut self, weight: f32) -> Self {
        self.weight = AnimatedF32::stable(weight);
        self
    }
}

// --- MARK: WIDGETMUT
impl VariableLabel {
    /// Returns the underlying label for this widget.
    pub fn label_mut<'t>(this: &'t mut WidgetMut<'_, Self>) -> WidgetMut<'t, Label> {
        this.ctx.get_mut(&mut this.widget.label)
    }

    /// Sets the text of this label.
    pub fn set_text(this: &mut WidgetMut<'_, Self>, new_text: impl Into<ArcStr>) {
        Label::set_text(&mut Self::label_mut(this), new_text);
    }

    /// Sets the weight which this font will target.
    pub fn set_target_weight(this: &mut WidgetMut<'_, Self>, target: f32, over_millis: f32) {
        this.widget.weight.move_to(target, over_millis);
        this.ctx.request_layout();
        this.ctx.request_anim_frame();
    }
}

// --- MARK: IMPL WIDGET
impl Widget for VariableLabel {
    type Action = NoAction;

    fn accepts_pointer_interaction(&self) -> bool {
        false
    }

    fn update(
        &mut self,
        _ctx: &mut UpdateCtx<'_>,
        _props: &mut PropertiesMut<'_>,
        _event: &Update,
    ) {
    }

    fn register_children(&mut self, ctx: &mut RegisterCtx<'_>) {
        ctx.register_child(&mut self.label);
    }

    fn on_anim_frame(
        &mut self,
        ctx: &mut UpdateCtx<'_>,
        _props: &mut PropertiesMut<'_>,
        interval: u64,
    ) {
        let millis = (interval as f64 / 1_000_000.) as f32;
        let result = self.weight.advance(millis);
        let new_weight = self.weight.value;
        // The ergonomics of child widgets are quite bad - ideally, this wouldn't need a mutate pass, since we
        // can set the required invalidation anyway.
        ctx.mutate_child_later(&mut self.label, move |mut label| {
            // TODO: Should this be configurable?
            if result.is_completed() {
                Label::set_hint(&mut label, true);
            } else {
                Label::set_hint(&mut label, false);
            }
            Label::insert_style(
                &mut label,
                StyleProperty::FontWeight(FontWeight::new(new_weight)),
            );
        });
        if !result.is_completed() {
            ctx.request_anim_frame();
        }
    }

    fn measure(
        &mut self,
        ctx: &mut MeasureCtx<'_>,
        _props: &PropertiesRef<'_>,
        axis: Axis,
        _len_req: LenReq,
        cross_length: Option<f64>,
    ) -> f64 {
        ctx.redirect_measurement(&mut self.label, axis, cross_length)
    }

    fn layout(&mut self, ctx: &mut LayoutCtx<'_>, _props: &PropertiesRef<'_>, size: Size) {
        ctx.run_layout(&mut self.label, size);
        ctx.place_child(&mut self.label, Point::ORIGIN);

        let child_baseline = ctx.child_baseline_offset(&self.label);
        ctx.set_baseline_offset(child_baseline);
    }

    fn paint(&mut self, _ctx: &mut PaintCtx<'_>, _props: &PropertiesRef<'_>, _scene: &mut Scene) {}

    fn accessibility_role(&self) -> Role {
        Role::GenericContainer
    }

    fn accessibility(
        &mut self,
        _ctx: &mut AccessCtx<'_>,
        _props: &PropertiesRef<'_>,
        _node: &mut Node,
    ) {
    }

    fn children_ids(&self) -> ChildrenIds {
        ChildrenIds::from_slice(&[self.label.id()])
    }

    fn make_trace_span(&self, id: WidgetId) -> Span {
        trace_span!("VariableLabel", id = id.trace())
    }
}

// --- MARK: TESTS
#[cfg(test)]
mod tests {
    // TODO - Add tests
}
