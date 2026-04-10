// Copyright 2026 the Xilem Authors
// SPDX-License-Identifier: Apache-2.0

use std::sync::Arc;

use masonry::core::ArcStr;
use masonry::properties::ObjectFit;
use masonry::widgets;
use usvg::Tree;

use crate::core::{MessageCtx, MessageResult, Mut, View, ViewMarker};
use crate::view::Prop;
use crate::{Pod, ViewCtx, WidgetView};

/// Displays the SVG.
///
/// By default, the SVG will be scaled to fully fit within the container ([`ObjectFit::Contain`]).
/// To configure this, call [`fit`](Svg::fit) on the returned value.
pub fn svg(tree: Arc<Tree>) -> Svg {
    Svg {
        tree,
        decorative: false,
        alt_text: None,
    }
}

/// A view of an SVG image.
#[must_use = "View values do nothing unless provided to Xilem."]
pub struct Svg {
    tree: Arc<Tree>,
    decorative: bool,
    alt_text: Option<ArcStr>,
}

impl Svg {
    /// Specify the object fit.
    pub fn fit<State: 'static, Action: 'static>(
        self,
        fill: ObjectFit,
    ) -> Prop<ObjectFit, Self, State, Action> {
        self.prop(fill)
    }

    /// Specifies whether the SVG is decorative, meaning it doesn't have meaningful content
    /// and is only for visual presentation.
    ///
    /// If `is_decorative` is `true`, the SVG will be ignored by screen readers.
    pub fn decorative(mut self, is_decorative: bool) -> Self {
        self.decorative = is_decorative;
        self
    }

    /// Sets the text that will describe the SVG to screen readers.
    ///
    /// Users are encouraged to set alt text for the SVG.
    /// If possible, the alt-text should succinctly describe what the SVG represents.
    ///
    /// If the SVG is decorative users should set alt text to `""`.
    /// If it's too hard to describe through text, the alt text should be left unset.
    /// This allows accessibility clients to know that there is no accessible description of the SVG content.
    pub fn with_alt_text(mut self, alt_text: impl Into<ArcStr>) -> Self {
        self.alt_text = Some(alt_text.into());
        self
    }
}
impl ViewMarker for Svg {}
impl<State: 'static, Action> View<State, Action, ViewCtx> for Svg {
    type Element = Pod<widgets::Svg>;
    type ViewState = ();

    fn build(&self, ctx: &mut ViewCtx, _: &mut State) -> (Self::Element, Self::ViewState) {
        let mut svg = widgets::Svg::new(self.tree.clone()).decorative(self.decorative);
        if let Some(alt_text) = &self.alt_text {
            svg = svg.with_alt_text(alt_text.clone());
        }
        (ctx.create_pod(svg), ())
    }

    fn rebuild(
        &self,
        prev: &Self,
        (): &mut Self::ViewState,
        _: &mut ViewCtx,
        mut element: Mut<'_, Self::Element>,
        _: &mut State,
    ) {
        if !Arc::ptr_eq(&prev.tree, &self.tree) {
            widgets::Svg::set_tree(&mut element, self.tree.clone());
        }
        if self.decorative != prev.decorative {
            widgets::Svg::set_decorative(&mut element, self.decorative);
        }
        if self.alt_text != prev.alt_text {
            widgets::Svg::set_alt_text(&mut element, self.alt_text.clone());
        }
    }

    fn teardown(&self, (): &mut Self::ViewState, _: &mut ViewCtx, _: Mut<'_, Self::Element>) {}

    fn message(
        &self,
        (): &mut Self::ViewState,
        message: &mut MessageCtx,
        _: Mut<'_, Self::Element>,
        _: &mut State,
    ) -> MessageResult<Action> {
        tracing::error!(
            ?message,
            "Message arrived in Svg::message, but Svg doesn't consume any messages, this is a bug."
        );
        MessageResult::Stale
    }
}
