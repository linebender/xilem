// Copyright 2024 the Xilem Authors
// SPDX-License-Identifier: Apache-2.0

use masonry::core::ArcStr;
use masonry::parley::style::{FontStack, FontWeight};
use masonry::widgets;
use vello::peniko::Brush;
use xilem_core::ViewPathTracker;

use super::{Label, label};
use crate::core::{DynMessage, Mut, ViewMarker};
use crate::{MessageResult, Pod, TextAlign, View, ViewCtx, ViewId};

/// A view for displaying non-editable text, with a variable [weight](masonry::parley::style::FontWeight).
pub fn variable_label(text: impl Into<ArcStr>) -> VariableLabel {
    VariableLabel {
        label: label(text),
        target_weight: FontWeight::NORMAL,
        over_millis: 0.,
    }
}

/// The [`View`] created by [`variable_label`].
#[must_use = "View values do nothing unless provided to Xilem."]
pub struct VariableLabel {
    label: Label,
    target_weight: FontWeight,
    over_millis: f32,
}

impl VariableLabel {
    /// Set the weight this label will target.
    ///
    /// If this change is animated, it will occur over `over_millis` milliseconds.
    /// Note that this will also be used as the initial font weight when the label is
    /// first created.
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
        self.target_weight = FontWeight::new(weight);
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
    pub fn font(mut self, font: impl Into<FontStack<'static>>) -> Self {
        self.label = self.label.font(font);
        self
    }

    /// Set the brush used to paint the text.
    #[doc(alias = "color")]
    pub fn brush(mut self, brush: impl Into<Brush>) -> Self {
        self.label = self.label.brush(brush);
        self
    }

    /// Set the [text alignment](https://en.wikipedia.org/wiki/Typographic_alignment) of the text.
    pub fn text_alignment(mut self, text_alignment: TextAlign) -> Self {
        self.label = self.label.text_alignment(text_alignment);
        self
    }

    /// Set the font size of the text.
    #[doc(alias = "font_size")]
    pub fn text_size(mut self, text_size: f32) -> Self {
        self.label = self.label.text_size(text_size);
        self
    }
}

impl ViewMarker for VariableLabel {}
impl<State, Action> View<State, Action, ViewCtx> for VariableLabel {
    type Element = Pod<widgets::VariableLabel>;
    type ViewState = ();

    fn build(&self, ctx: &mut ViewCtx, app_state: &mut State) -> (Self::Element, Self::ViewState) {
        let (label, ()) = ctx.with_id(ViewId::new(0), |ctx| {
            View::<State, Action, _>::build(&self.label, ctx, app_state)
        });
        let widget_pod = ctx.create_pod(
            widgets::VariableLabel::from_label_pod(label.into_widget_pod())
                .with_initial_weight(self.target_weight.value()),
        );
        (widget_pod, ())
    }

    fn rebuild(
        &self,
        prev: &Self,
        (): &mut Self::ViewState,
        ctx: &mut ViewCtx,
        mut element: Mut<'_, Self::Element>,
        app_state: &mut State,
    ) {
        ctx.with_id(ViewId::new(0), |ctx| {
            View::<State, Action, _>::rebuild(
                &self.label,
                &prev.label,
                &mut (),
                ctx,
                widgets::VariableLabel::label_mut(&mut element),
                app_state,
            );
        });

        if prev.target_weight != self.target_weight {
            widgets::VariableLabel::set_target_weight(
                &mut element,
                self.target_weight.value(),
                self.over_millis,
            );
        }
    }

    fn teardown(
        &self,
        (): &mut Self::ViewState,
        ctx: &mut ViewCtx,
        mut element: Mut<'_, Self::Element>,
        app_state: &mut State,
    ) {
        ctx.with_id(ViewId::new(0), |ctx| {
            View::<State, Action, _>::teardown(
                &self.label,
                &mut (),
                ctx,
                widgets::VariableLabel::label_mut(&mut element),
                app_state,
            );
        });
    }

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
            tracing::error!(
                "Message arrived in VariableLabel::message, but VariableLabel doesn't consume any messages, this is a bug"
            );
            MessageResult::Stale(message)
        }
    }
}
