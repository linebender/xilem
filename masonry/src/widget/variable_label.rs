// Copyright 2024 the Xilem Authors and the Druid Authors
// SPDX-License-Identifier: Apache-2.0

//! A label with support for animated variable font properties
// Copyright 2019 the Xilem Authors and the Druid Authors
// SPDX-License-Identifier: Apache-2.0

//! A label widget.

use accesskit::Role;
use kurbo::{Affine, Point, Size};
use parley::fontique::Weight;
use parley::layout::Alignment;
use parley::style::{FontFamily, FontStack};
use smallvec::SmallVec;
use tracing::{trace, trace_span, Span};
use vello::peniko::BlendMode;
use vello::Scene;

use crate::text::{TextBrush, TextLayout, TextStorage};
use crate::widget::WidgetMut;
use crate::{
    AccessCtx, AccessEvent, ArcStr, BoxConstraints, EventCtx, LayoutCtx, LifeCycle, LifeCycleCtx,
    PaintCtx, PointerEvent, StatusChange, TextEvent, Widget, WidgetId,
};

use super::LineBreaking;

// added padding between the edges of the widget and the text.
pub(super) const LABEL_X_PADDING: f64 = 2.0;

pub struct AnimatedFloat {
    target: f32,
    value: f32,
    // TODO: Provide different easing functions
    rate_per_millisecond: f32,
}

impl AnimatedFloat {
    pub fn stable(value: f32) -> Self {
        assert!(value.is_finite());
        AnimatedFloat {
            target: value,
            value,
            rate_per_millisecond: 0.,
        }
    }

    pub fn move_to(&mut self, target: f32, over_millis: f32) {
        assert!(target.is_finite());
        self.target = target;
        self.rate_per_millisecond = (self.value - self.target) / over_millis;
        assert!(
            self.rate_per_millisecond.is_finite(),
            "Provided invalid time step {over_millis}"
        );
    }

    pub fn advance(&mut self, by_millis: f32) {
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
        }
    }
}

/// A widget displaying non-editable text.
pub struct VariableLabel {
    text_layout: TextLayout<ArcStr>,
    line_break_mode: LineBreaking,
    show_disabled: bool,
    brush: TextBrush,
    weight: AnimatedFloat,
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
            weight: AnimatedFloat::stable(Weight::NORMAL.value()),
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

    /// Create a label with empty text.
    pub fn empty() -> Self {
        Self::new("")
    }

    pub fn with_initial_weight(&mut self) {}
}

// --- MARK: WIDGETMUT ---
impl WidgetMut<'_, VariableLabel> {
    pub fn text(&self) -> &ArcStr {
        self.widget.text_layout.text()
    }

    pub fn set_text_properties<R>(&mut self, f: impl FnOnce(&mut TextLayout<ArcStr>) -> R) -> R {
        let ret = f(&mut self.widget.text_layout);
        if self.widget.text_layout.needs_rebuild() {
            self.ctx.request_layout();
            self.ctx.request_paint();
        }
        ret
    }

    pub fn set_text(&mut self, new_text: impl Into<ArcStr>) {
        let new_text = new_text.into();
        self.set_text_properties(|layout| layout.set_text(new_text));
    }

    #[doc(alias = "set_text_color")]
    pub fn set_text_brush(&mut self, brush: impl Into<TextBrush>) {
        let brush = brush.into();
        self.widget.brush = brush;
        if !self.ctx.is_disabled() {
            let brush = self.widget.brush.clone();
            self.set_text_properties(|layout| layout.set_brush(brush));
        }
    }
    pub fn set_text_size(&mut self, size: f32) {
        self.set_text_properties(|layout| layout.set_text_size(size));
    }
    pub fn set_alignment(&mut self, alignment: Alignment) {
        self.set_text_properties(|layout| layout.set_text_alignment(alignment));
    }
    pub fn set_font(&mut self, font_stack: FontStack<'static>) {
        self.set_text_properties(|layout| layout.set_font(font_stack));
    }
    pub fn set_font_family(&mut self, family: FontFamily<'static>) {
        self.set_font(FontStack::Single(family));
    }
    pub fn set_line_break_mode(&mut self, line_break_mode: LineBreaking) {
        self.widget.line_break_mode = line_break_mode;
        self.ctx.request_paint();
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
            LifeCycle::BuildFocusChain => {
                if !self.text_layout.text().links().is_empty() {
                    tracing::warn!("Links present in text, but not yet integrated");
                }
            }
            LifeCycle::AnimFrame(time) => {
                let millis = (*time as f64 / 1000.) as f32;
                self.weight.advance(millis);
                self.text_layout.needs_rebuild();
                ctx.request_anim_frame();
                ctx.request_layout();
                ctx.request_paint();
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
        } else if bc.min().width.is_sign_negative() {
            Some(0.0)
        } else {
            None
        };
        self.text_layout.set_max_advance(max_advance);
        if self.text_layout.needs_rebuild() {
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

    fn accessibility(&mut self, ctx: &mut AccessCtx) {
        ctx.current_node()
            .set_name(self.text().as_str().to_string());
    }

    fn children_ids(&self) -> SmallVec<[WidgetId; 16]> {
        SmallVec::new()
    }

    fn make_trace_span(&self) -> Span {
        trace_span!("Label")
    }

    fn get_debug_text(&self) -> Option<String> {
        Some(self.text_layout.text().as_str().to_string())
    }
}

// --- MARK: TESTS ---
#[cfg(test)]
mod tests {}
