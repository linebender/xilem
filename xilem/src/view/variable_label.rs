// Copyright 2024 the Xilem Authors
// SPDX-License-Identifier: Apache-2.0

use masonry::parley::fontique::Weight;
use masonry::parley::style::FontStack;
use masonry::text::ArcStr;
use masonry::{widget, TextAlignment};
use vello::peniko::Brush;
use xilem_core::ViewPathTracker;

use crate::core::{DynMessage, Mut, ViewMarker};
use crate::{MessageResult, Pod, TextWeight, View, ViewCtx, ViewId};

use super::{label, Label};

/// A view for displaying non-editable text, with a variable [weight](masonry::parley::style::FontWeight).
pub fn variable_label(text: impl Into<ArcStr>) -> VariableLabel {
    VariableLabel {
        label: label(text),
        target_weight: Weight::NORMAL,
        over_millis: 0.,
    }
}

#[must_use = "View values do nothing unless provided to Xilem."]
pub struct VariableLabel {
    label: Label,
    target_weight: Weight,
    over_millis: f32,
}

impl VariableLabel {
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
    /// This should be a font stack with variable font support,
    /// although non-variable fonts will work, just without the smooth animation support.
    pub fn with_font(mut self, font: impl Into<FontStack<'static>>) -> Self {
        self.label.font = font.into();
        self
    }

    #[doc(alias = "color")]
    pub fn brush(mut self, brush: impl Into<Brush>) -> Self {
        self.label.text_brush = brush.into();
        self
    }

    pub fn alignment(mut self, alignment: TextAlignment) -> Self {
        self.label.alignment = alignment;
        self
    }

    #[doc(alias = "font_size")]
    pub fn text_size(mut self, text_size: f32) -> Self {
        self.label.text_size = text_size;
        self
    }

    pub fn weight(mut self, weight: TextWeight) -> Self {
        self.label.weight = weight;
        self
    }
}

impl VariableLabel {}

impl ViewMarker for VariableLabel {}
impl<State, Action> View<State, Action, ViewCtx> for VariableLabel {
    type Element = Pod<widget::VariableLabel>;
    type ViewState = ();

    fn build(&self, ctx: &mut ViewCtx) -> (Self::Element, Self::ViewState) {
        let (label, ()) = ctx.with_id(ViewId::new(0), |ctx| {
            View::<State, Action, _, _>::build(&self.label, ctx)
        });
        let widget_pod = ctx.new_pod(
            widget::VariableLabel::from_label_pod(label.inner)
                .with_initial_weight(self.target_weight.value()),
        );
        (widget_pod, ())
    }

    fn rebuild(
        &self,
        prev: &Self,
        (): &mut Self::ViewState,
        ctx: &mut ViewCtx,
        mut element: Mut<Self::Element>,
    ) {
        ctx.with_id(ViewId::new(0), |ctx| {
            View::<State, Action, _, _>::rebuild(
                &self.label,
                &prev.label,
                &mut (),
                ctx,
                widget::VariableLabel::label_mut(&mut element),
            );
        });

        if prev.target_weight != self.target_weight {
            widget::VariableLabel::set_target_weight(
                &mut element,
                self.target_weight.value(),
                self.over_millis,
            );
        }
    }

    fn teardown(&self, (): &mut Self::ViewState, _: &mut ViewCtx, _: Mut<Self::Element>) {}

    fn message(
        &self,
        (): &mut Self::ViewState,
        id_path: &[ViewId],
        message: DynMessage,
        app_state: &mut State,
    ) -> MessageResult<Action> {
        if let Some((first, remainder)) = id_path.split_first() {
            assert_eq!(first.routing_id(), 0);
            self.label.message(&mut (), remainder, message, app_state)
        } else {
            tracing::error!("Message arrived in Label::message, but Label doesn't consume any messages, this is a bug");
            MessageResult::Stale(message)
        }
    }
}
