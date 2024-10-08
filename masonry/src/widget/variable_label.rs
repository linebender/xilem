// Copyright 2019 the Xilem Authors and the Druid Authors
// SPDX-License-Identifier: Apache-2.0

//! A label with support for animated variable font properties

use std::cmp::Ordering;

use accesskit::{NodeBuilder, Role};
use parley::fontique::Weight;
use parley::layout::Alignment;
use parley::style::{FontFamily, FontStack};
use smallvec::SmallVec;
use tracing::{trace, trace_span, Span};
use vello::kurbo::{Affine, Point, Size};
use vello::peniko::BlendMode;
use vello::Scene;

use crate::text::{Hinting, TextBrush, TextLayout};
use crate::widget::{LineBreaking, WidgetMut};
use crate::{
    AccessCtx, AccessEvent, ArcStr, BoxConstraints, EventCtx, LayoutCtx, LifeCycle, LifeCycleCtx,
    PaintCtx, PointerEvent, RegisterCtx, StatusChange, TextEvent, Widget, WidgetId,
};

// added padding between the edges of the widget and the text.
pub(super) const LABEL_X_PADDING: f64 = 2.0;

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

    /// Is this animation finished?
    pub fn is_completed(&self) -> bool {
        self.target == self.value
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

// TODO: Make this a wrapper (around `Label`?)
/// A widget displaying non-editable text, with a variable [weight](parley::style::FontWeight).
pub struct VariableLabel {
    text_layout: TextLayout<ArcStr>,
    line_break_mode: LineBreaking,
    show_disabled: bool,
    brush: TextBrush,
    weight: AnimatedF32,
}

// --- MARK: BUILDERS ---
impl VariableLabel {
    /// Create a new label.
    pub fn new(text: impl Into<ArcStr>) -> Self {
        Self {
            text_layout: TextLayout::new(text.into(), crate::theme::TEXT_SIZE_NORMAL as f32),
            line_break_mode: LineBreaking::Overflow,
            show_disabled: true,
            brush: crate::theme::TEXT_COLOR.into(),
            weight: AnimatedF32::stable(Weight::NORMAL.value()),
        }
    }

    pub fn text(&self) -> &ArcStr {
        self.text_layout.text()
    }

    #[doc(alias = "with_text_color")]
    pub fn with_text_brush(mut self, brush: impl Into<TextBrush>) -> Self {
        self.text_layout.set_brush(brush);
        self
    }

    #[doc(alias = "with_font_size")]
    pub fn with_text_size(mut self, size: f32) -> Self {
        self.text_layout.set_text_size(size);
        self
    }

    pub fn with_text_alignment(mut self, alignment: Alignment) -> Self {
        self.text_layout.set_text_alignment(alignment);
        self
    }

    pub fn with_font(mut self, font: FontStack<'static>) -> Self {
        self.text_layout.set_font(font);
        self
    }
    pub fn with_font_family(self, font: FontFamily<'static>) -> Self {
        self.with_font(FontStack::Single(font))
    }

    pub fn with_line_break_mode(mut self, line_break_mode: LineBreaking) -> Self {
        self.line_break_mode = line_break_mode;
        self
    }
    /// Set the initial font weight for this text.
    pub fn with_initial_weight(mut self, weight: f32) -> Self {
        self.weight = AnimatedF32::stable(weight);
        self
    }

    /// Create a label with empty text.
    pub fn empty() -> Self {
        Self::new("")
    }

    fn brush(&self, disabled: bool) -> TextBrush {
        if disabled {
            crate::theme::DISABLED_TEXT_COLOR.into()
        } else {
            let mut brush = self.brush.clone();
            if !self.weight.is_completed() {
                brush.set_hinting(Hinting::No);
            }
            // N.B. if hinting is No externally, we don't want to overwrite it to yes.
            brush
        }
    }
}

// --- MARK: WIDGETMUT ---
impl WidgetMut<'_, VariableLabel> {
    /// Read the text.
    pub fn text(&self) -> &ArcStr {
        self.widget.text_layout.text()
    }

    /// Set a property on the underlying text.
    ///
    /// This cannot be used to set attributes.
    pub fn set_text_properties<R>(&mut self, f: impl FnOnce(&mut TextLayout<ArcStr>) -> R) -> R {
        let ret = f(&mut self.widget.text_layout);
        if self.widget.text_layout.needs_rebuild() {
            self.ctx.request_layout();
        }
        ret
    }

    /// Modify the underlying text.
    pub fn set_text(&mut self, new_text: impl Into<ArcStr>) {
        let new_text = new_text.into();
        self.set_text_properties(|layout| layout.set_text(new_text));
    }

    #[doc(alias = "set_text_color")]
    /// Set the brush of the text, normally used for the colour.
    pub fn set_text_brush(&mut self, brush: impl Into<TextBrush>) {
        let brush = brush.into();
        self.widget.brush = brush;
        if !self.ctx.is_disabled() {
            self.widget.text_layout.invalidate();
            self.ctx.request_layout();
        }
    }
    /// Set the font size for this text.
    pub fn set_text_size(&mut self, size: f32) {
        self.set_text_properties(|layout| layout.set_text_size(size));
    }
    /// Set the text alignment of the contained text
    pub fn set_alignment(&mut self, alignment: Alignment) {
        self.set_text_properties(|layout| layout.set_text_alignment(alignment));
    }
    /// Set the font (potentially with fallbacks) which will be used for this text.
    pub fn set_font(&mut self, font_stack: FontStack<'static>) {
        self.set_text_properties(|layout| layout.set_font(font_stack));
    }
    /// A helper method to use a single font family.
    pub fn set_font_family(&mut self, family: FontFamily<'static>) {
        self.set_font(FontStack::Single(family));
    }
    /// How to handle overflowing lines.
    pub fn set_line_break_mode(&mut self, line_break_mode: LineBreaking) {
        self.widget.line_break_mode = line_break_mode;
        self.ctx.request_layout();
    }
    /// Set the weight which this font will target.
    pub fn set_target_weight(&mut self, target: f32, over_millis: f32) {
        self.widget.weight.move_to(target, over_millis);
        self.ctx.request_layout();
        self.ctx.request_anim_frame();
    }
}

// --- MARK: IMPL WIDGET ---
impl Widget for VariableLabel {
    fn on_pointer_event(&mut self, _ctx: &mut EventCtx, event: &PointerEvent) {
        match event {
            PointerEvent::PointerMove(_point) => {
                // TODO: Set cursor if over link
            }
            PointerEvent::PointerDown(_button, _state) => {
                // TODO: Start tracking currently pressed
                // (i.e. don't press)
            }
            PointerEvent::PointerUp(_button, _state) => {
                // TODO: Follow link (if not now dragging ?)
            }
            _ => {}
        }
    }

    fn on_text_event(&mut self, _ctx: &mut EventCtx, _event: &TextEvent) {
        // If focused on a link and enter pressed, follow it?
        // TODO: This sure looks like each link needs its own widget, although I guess the challenge there is
        // that the bounding boxes can go e.g. across line boundaries?
    }

    fn on_access_event(&mut self, _ctx: &mut EventCtx, _event: &AccessEvent) {}

    fn register_children(&mut self, _ctx: &mut RegisterCtx) {}

    #[allow(missing_docs)]
    fn on_status_change(&mut self, _ctx: &mut LifeCycleCtx, event: &StatusChange) {
        match event {
            StatusChange::FocusChanged(_) => {
                // TODO: Focus on first link
            }
            _ => {}
        }
    }

    fn lifecycle(&mut self, ctx: &mut LifeCycleCtx, event: &LifeCycle) {
        match event {
            LifeCycle::DisabledChanged(disabled) => {
                if self.show_disabled {
                    if *disabled {
                        self.text_layout
                            .set_brush(crate::theme::DISABLED_TEXT_COLOR);
                    } else {
                        self.text_layout.set_brush(self.brush.clone());
                    }
                }
                // TODO: Parley seems to require a relayout when colours change
                ctx.request_layout();
            }
            LifeCycle::AnimFrame(time) => {
                let millis = (*time as f64 / 1_000_000.) as f32;
                let result = self.weight.advance(millis);
                self.text_layout.invalidate();
                if !result.is_completed() {
                    ctx.request_anim_frame();
                }
                ctx.request_layout();
            }
            _ => {}
        }
    }

    fn layout(&mut self, ctx: &mut LayoutCtx, bc: &BoxConstraints) -> Size {
        // Compute max_advance from box constraints
        let max_advance = if self.line_break_mode != LineBreaking::WordWrap {
            None
        } else if bc.max().width.is_finite() {
            Some(bc.max().width as f32 - 2. * LABEL_X_PADDING as f32)
        } else {
            None
        };
        self.text_layout.set_max_advance(max_advance);
        if self.text_layout.needs_rebuild() {
            self.text_layout
                .set_brush(self.brush(ctx.widget_state.is_disabled));
            let (font_ctx, layout_ctx) = ctx.text_contexts();
            self.text_layout
                .rebuild_with_attributes(font_ctx, layout_ctx, |mut builder| {
                    builder.push_default(&parley::style::StyleProperty::FontWeight(Weight::new(
                        self.weight.value,
                    )));
                    // builder.push_default(&parley::style::StyleProperty::FontVariations(
                    //     parley::style::FontSettings::List(&[]),
                    // ));
                    builder
                });
        }
        // We ignore trailing whitespace for a label
        let text_size = self.text_layout.size();
        let label_size = Size {
            height: text_size.height,
            width: text_size.width + 2. * LABEL_X_PADDING,
        };
        let size = bc.constrain(label_size);
        trace!(
            "Computed layout: max={:?}. w={}, h={}",
            max_advance,
            size.width,
            size.height,
        );
        size
    }

    fn paint(&mut self, ctx: &mut PaintCtx, scene: &mut Scene) {
        if self.text_layout.needs_rebuild() {
            debug_panic!("Called Label paint before layout");
        }
        if self.line_break_mode == LineBreaking::Clip {
            let clip_rect = ctx.size().to_rect();
            scene.push_layer(BlendMode::default(), 1., Affine::IDENTITY, &clip_rect);
        }
        self.text_layout
            .draw(scene, Point::new(LABEL_X_PADDING, 0.0));

        if self.line_break_mode == LineBreaking::Clip {
            scene.pop_layer();
        }
    }

    fn accessibility_role(&self) -> Role {
        Role::Label
    }

    fn accessibility(&mut self, _ctx: &mut AccessCtx, node: &mut NodeBuilder) {
        node.set_name(self.text().as_ref().to_string());
    }

    fn children_ids(&self) -> SmallVec<[WidgetId; 16]> {
        SmallVec::new()
    }

    fn make_trace_span(&self) -> Span {
        trace_span!("VariableLabel")
    }

    fn get_debug_text(&self) -> Option<String> {
        Some(self.text_layout.text().as_ref().to_string())
    }
}

// --- MARK: TESTS ---
#[cfg(test)]
mod tests {}
