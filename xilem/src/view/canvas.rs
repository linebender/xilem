// Copyright 2024 the Xilem Authors
// SPDX-License-Identifier: Apache-2.0

use masonry::core::ArcStr;
use masonry::widgets;
use vello::Scene;
use vello::kurbo::Size;

use crate::core::{Arg, MessageCtx, MessageResult, Mut, View, ViewArgument, ViewMarker};
use crate::{Pod, ViewCtx};

/// # Example
///
/// ```
/// use xilem::masonry::kurbo::{Rect, Size, Affine};
/// use xilem::masonry::peniko::Fill;
/// use xilem::masonry::vello::Scene;
/// use xilem::view::canvas;
/// use std::sync::Arc;
/// # use xilem::WidgetView;
///
/// # fn fill_canvas() -> impl WidgetView<()> + use<> {
/// let my_canvas = canvas(|scene: &mut Scene, size: Size| {
///     // Drawing a simple rectangle that fills the canvas.
///     scene.fill(
///         Fill::NonZero,
///         Affine::IDENTITY,
///         xilem::palette::css::AQUA,
///         None,
///         &Rect::new(0.0, 0.0, size.width, size.height),
///     );
/// });
/// # my_canvas
/// # }
/// ```
pub fn canvas(draw: fn(&mut Scene, Size)) -> Canvas {
    Canvas {
        draw,
        alt_text: ArcStr::default(),
    }
}

/// The [`View`] created by [`canvas`].
#[must_use = "View values do nothing unless provided to Xilem."]
pub struct Canvas {
    draw: fn(&mut Scene, Size),
    alt_text: ArcStr,
}

impl Canvas {
    /// Sets alt text for the contents of the canvas.
    ///
    /// Users are strongly encouraged to provide alt text for accessibility tools
    /// to use.
    pub fn alt_text(mut self, alt_text: impl Into<ArcStr>) -> Self {
        self.alt_text = alt_text.into();
        self
    }
}

impl ViewMarker for Canvas {}

impl<State: ViewArgument, Action> View<State, Action, ViewCtx> for Canvas {
    type Element = Pod<widgets::Canvas>;
    type ViewState = ();

    fn build(&self, ctx: &mut ViewCtx, _: Arg<'_, State>) -> (Self::Element, Self::ViewState) {
        let widget_pod = ctx.create_pod(widgets::Canvas::new(self.draw, self.alt_text.clone()));
        (widget_pod, ())
    }

    fn rebuild(
        &self,
        prev: &Self,
        (): &mut Self::ViewState,
        _ctx: &mut ViewCtx,
        mut element: Mut<'_, Self::Element>,
        _: Arg<'_, State>,
    ) {
        if !std::ptr::fn_addr_eq(self.draw, prev.draw) {
            widgets::Canvas::set_draw(&mut element, self.draw);
        }
        if self.alt_text != prev.alt_text {
            widgets::Canvas::set_alt_text(element, self.alt_text.clone());
        }
    }

    fn teardown(&self, (): &mut Self::ViewState, _: &mut ViewCtx, _: Mut<'_, Self::Element>) {}

    fn message(
        &self,
        (): &mut Self::ViewState,
        message: &mut MessageCtx,
        _element: Mut<'_, Self::Element>,
        _app_state: Arg<'_, State>,
    ) -> MessageResult<Action> {
        tracing::error!(
            ?message,
            "Message arrived in Canvas::message, but Canvas doesn't consume any messages, this is a bug"
        );
        MessageResult::Stale
    }
}
