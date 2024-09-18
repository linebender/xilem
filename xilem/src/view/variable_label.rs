// Copyright 2024 the Xilem Authors
// SPDX-License-Identifier: Apache-2.0

use masonry::{
    parley::{
        fontique::Weight,
        style::{FontFamily, FontStack, GenericFamily},
    },
    text::TextBrush,
    widget, ArcStr,
};
use xilem_core::{Mut, ViewMarker};

use crate::{Color, MessageResult, Pod, TextAlignment, View, ViewCtx, ViewId};

/// A view for displaying non-editable text, with a variable [weight](masonry::parley::style::FontWeight).
pub fn variable_label(label: impl Into<ArcStr>) -> VariableLabel {
    VariableLabel {
        label: label.into(),
        text_brush: Color::WHITE.into(),
        alignment: TextAlignment::default(),
        text_size: masonry::theme::TEXT_SIZE_NORMAL as f32,
        target_weight: Weight::NORMAL,
        over_millis: 0.,
        font: FontStack::Single(FontFamily::Generic(GenericFamily::SystemUi)),
    }
}

pub struct VariableLabel {
    label: ArcStr,

    text_brush: TextBrush,
    alignment: TextAlignment,
    text_size: f32,
    target_weight: Weight,
    over_millis: f32,
    font: FontStack<'static>,
    // TODO: add more attributes of `masonry::widget::Label`
}

impl VariableLabel {
    #[doc(alias = "color")]
    pub fn brush(mut self, brush: impl Into<TextBrush>) -> Self {
        self.text_brush = brush.into();
        self
    }

    pub fn alignment(mut self, alignment: TextAlignment) -> Self {
        self.alignment = alignment;
        self
    }

    #[doc(alias = "font_size")]
    pub fn text_size(mut self, text_size: f32) -> Self {
        self.text_size = text_size;
        self
    }

    /// Set the weight this label will target.
    ///
    /// If this change is animated, it will occur over `over_millis` milliseconds.
    ///
    /// Note that updating `over_millis` without changing `weight` will *not* change
    /// the length of time the weight change occurs over.
    ///
    /// `over_millis` should be non-negative.
    /// `weight` should be within the valid range for font weights.
    ///
    /// # Panics
    ///
    /// If `weight` is non-finite.
    pub fn target_weight(mut self, weight: f32, over_millis: f32) -> Self {
        assert!(weight.is_finite(), "Invalid target weight {weight}.");
        self.target_weight = Weight::new(weight);
        self.over_millis = over_millis;
        self
    }

    /// Set the [font stack](FontStack) this label will use.
    ///
    /// A font stack allows for providing fallbacks. If there is no matching font
    /// for a character, a system font will be used (if the system fonts are enabled).
    ///
    /// This currently requires a `FontStack<'static>`, because it is stored in
    /// the view, and Parley doesn't support an owned or `Arc` based `FontStack`.
    /// In most cases, a fontstack value can be static-promoted, but otherwise
    /// you will currently have to [leak](String::leak) a value and manually keep
    /// the value.
    ///
    /// This should be a font stack with variable font support,
    /// although non-variable fonts will work, just without the smooth animation support.
    pub fn with_font(mut self, font: FontStack<'static>) -> Self {
        self.font = font;
        self
    }
}

impl ViewMarker for VariableLabel {}
impl<State, Action> View<State, Action, ViewCtx> for VariableLabel {
    type Element = Pod<widget::VariableLabel>;
    type ViewState = ();

    fn build(&self, ctx: &mut ViewCtx) -> (Self::Element, Self::ViewState) {
        let widget_pod = ctx.new_pod(
            widget::VariableLabel::new(self.label.clone())
                .with_text_brush(self.text_brush.clone())
                .with_line_break_mode(widget::LineBreaking::WordWrap)
                .with_text_alignment(self.alignment)
                .with_font(self.font)
                .with_text_size(self.text_size)
                .with_initial_weight(self.target_weight.value()),
        );
        (widget_pod, ())
    }

    fn rebuild<'el>(
        &self,
        prev: &Self,
        (): &mut Self::ViewState,
        ctx: &mut ViewCtx,
        mut element: Mut<'el, Self::Element>,
    ) -> Mut<'el, Self::Element> {
        if prev.label != self.label {
            element.set_text(self.label.clone());
            ctx.mark_changed();
        }
        if prev.text_brush != self.text_brush {
            element.set_text_brush(self.text_brush.clone());
            ctx.mark_changed();
        }
        if prev.alignment != self.alignment {
            element.set_alignment(self.alignment);
            ctx.mark_changed();
        }
        if prev.text_size != self.text_size {
            element.set_text_size(self.text_size);
            ctx.mark_changed();
        }
        if prev.target_weight != self.target_weight {
            element.set_target_weight(self.target_weight.value(), self.over_millis);
            ctx.mark_changed();
        }
        // First perform a fast filter, then perform a full comparison if that suggests a possible change.
        let fonts_eq = fonts_eq_fastpath(prev.font, self.font) || prev.font == self.font;
        if !fonts_eq {
            element.set_font(self.font);
            ctx.mark_changed();
        }
        element
    }

    fn teardown(&self, (): &mut Self::ViewState, _: &mut ViewCtx, _: Mut<'_, Self::Element>) {}

    fn message(
        &self,
        (): &mut Self::ViewState,
        _id_path: &[ViewId],
        message: xilem_core::DynMessage,
        _app_state: &mut State,
    ) -> crate::MessageResult<Action> {
        tracing::error!("Message arrived in Label::message, but Label doesn't consume any messages, this is a bug");
        MessageResult::Stale(message)
    }
}

/// Because all the `FontStack`s we use are 'static, we expect the value to never change.
///
/// Because of this, we compare the inner pointer value first.
/// This function has false negatives, but no false positives.
///
/// It should be used with a secondary direct comparison using `==`
/// if it returns false. If the value does change, this is potentially more expensive.
fn fonts_eq_fastpath(lhs: FontStack<'static>, rhs: FontStack<'static>) -> bool {
    match (lhs, rhs) {
        (FontStack::Source(lhs), FontStack::Source(rhs)) => {
            // Slices/strs are properly compared by length
            core::ptr::eq(lhs.as_ptr(), rhs.as_ptr())
        }
        (FontStack::Single(FontFamily::Named(lhs)), FontStack::Single(FontFamily::Named(rhs))) => {
            core::ptr::eq(lhs.as_ptr(), rhs.as_ptr())
        }
        (FontStack::List(lhs), FontStack::List(rhs)) => core::ptr::eq(lhs.as_ptr(), rhs.as_ptr()),
        _ => false,
    }
}
