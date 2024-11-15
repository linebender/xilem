// Copyright 2019 the Xilem Authors and the Druid Authors
// SPDX-License-Identifier: Apache-2.0

//! A label with support for animated variable font properties

use std::cmp::Ordering;

use accesskit::{Node, Role};
use parley::fontique::Weight;
use parley::StyleProperty;
use smallvec::{smallvec, SmallVec};
use tracing::{trace_span, Span};
use vello::kurbo::{Point, Size};
use vello::Scene;

use crate::text::ArcStr;
use crate::widget::WidgetMut;
use crate::{
    AccessCtx, AccessEvent, BoxConstraints, EventCtx, LayoutCtx, PaintCtx, PointerEvent, QueryCtx,
    RegisterCtx, TextEvent, Update, UpdateCtx, Widget, WidgetId,
};

use super::{Label, WidgetPod};

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
    /// Create a value which is not changing.
    pub fn stable(value: f32) -> Self {
        assert!(value.is_finite());
        AnimatedF32 {
            target: value,
            value,
            rate_per_millisecond: 0.,
        }
    }

    /// Move this value to the `target` over `over_millis` milliseconds.
    /// Might change the current value, if `over_millis` is zero.
    ///
    /// `over_millis` should be non-negative.
    ///
    /// # Panics
    ///
    /// If `target` is not a finite value.
    pub fn move_to(&mut self, target: f32, over_millis: f32) {
        assert!(target.is_finite());
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

    /// Advance this animation by `by_millis` milliseconds.
    ///
    /// Returns the status of the animation after this advancement.
    pub fn advance(&mut self, by_millis: f32) -> AnimationStatus {
        if !self.value.is_finite() {
            tracing::error!("Got unexpected non-finite value {}", self.value);
            debug_assert!(self.target.is_finite());
            self.value = self.target;
        }

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
    pub fn is_completed(self) -> bool {
        matches!(self, AnimationStatus::Completed)
    }
}

/// A widget displaying non-editable text, with a variable [weight](parley::style::FontWeight).
pub struct VariableLabel {
    label: WidgetPod<Label>,
    weight: AnimatedF32,
}

// --- MARK: BUILDERS ---
impl VariableLabel {
    /// Create a new variable label from the given text.
    pub fn new(text: impl Into<ArcStr>) -> Self {
        Self::from_label_pod(WidgetPod::new(Label::new(text)))
    }

    pub fn from_label(label: Label) -> Self {
        Self::from_label_pod(WidgetPod::new(label))
    }

    pub fn from_label_pod(label: WidgetPod<Label>) -> Self {
        Self {
            label,
            weight: AnimatedF32::stable(Weight::NORMAL.value()),
        }
    }

    /// Set the initial font weight for this text.
    pub fn with_initial_weight(mut self, weight: f32) -> Self {
        self.weight = AnimatedF32::stable(weight);
        self
    }
}

// --- MARK: WIDGETMUT ---
impl VariableLabel {
    /// Get the underlying label for this widget.
    pub fn label_mut<'t>(this: &'t mut WidgetMut<'_, Self>) -> WidgetMut<'t, Label> {
        this.ctx.get_mut(&mut this.widget.label)
    }

    /// Set the text of this label.
    pub fn set_text(this: &mut WidgetMut<'_, Self>, new_text: impl Into<ArcStr>) {
        Label::set_text(&mut Self::label_mut(this), new_text);
    }

    /// Set the weight which this font will target.
    pub fn set_target_weight(this: &mut WidgetMut<'_, Self>, target: f32, over_millis: f32) {
        this.widget.weight.move_to(target, over_millis);
        this.ctx.request_layout();
        this.ctx.request_anim_frame();
    }
}

// --- MARK: IMPL WIDGET ---
impl Widget for VariableLabel {
    fn on_pointer_event(&mut self, _ctx: &mut EventCtx, _event: &PointerEvent) {}

    fn accepts_pointer_interaction(&self) -> bool {
        false
    }

    fn on_text_event(&mut self, _ctx: &mut EventCtx, _event: &TextEvent) {}

    fn on_access_event(&mut self, _ctx: &mut EventCtx, _event: &AccessEvent) {}

    fn update(&mut self, _ctx: &mut UpdateCtx, _event: &Update) {}

    fn register_children(&mut self, ctx: &mut RegisterCtx) {
        ctx.register_child(&mut self.label);
    }

    fn on_anim_frame(&mut self, ctx: &mut UpdateCtx, interval: u64) {
        let millis = (interval as f64 / 1_000_000.) as f32;
        let result = self.weight.advance(millis);
        let new_weight = self.weight.value;
        // The ergonomics of child widgets are quite bad - ideally, this wouldn't need a mutate pass, since we
        // can set the required invalidation anyway.
        ctx.mutate_later(&mut self.label, move |mut label| {
            // TODO: Should this be configurable?
            if result.is_completed() {
                Label::set_hint(&mut label, true);
            } else {
                Label::set_hint(&mut label, false);
            }
            Label::insert_style(
                &mut label,
                StyleProperty::FontWeight(Weight::new(new_weight)),
            );
        });
        if !result.is_completed() {
            ctx.request_anim_frame();
        }
    }

    fn layout(&mut self, ctx: &mut LayoutCtx, bc: &BoxConstraints) -> Size {
        let size = ctx.run_layout(&mut self.label, bc);
        ctx.place_child(&mut self.label, Point::ORIGIN);
        size
    }

    fn paint(&mut self, _ctx: &mut PaintCtx, _scene: &mut Scene) {}

    fn accessibility_role(&self) -> Role {
        Role::GenericContainer
    }

    fn accessibility(&mut self, _ctx: &mut AccessCtx, _node: &mut Node) {}

    fn children_ids(&self) -> SmallVec<[WidgetId; 16]> {
        smallvec![self.label.id()]
    }

    fn make_trace_span(&self, ctx: &QueryCtx<'_>) -> Span {
        trace_span!("VariableLabel", id = ctx.widget_id().trace())
    }
}

// --- MARK: TESTS ---
#[cfg(test)]
mod tests {
    // TODO - Add tests
}
