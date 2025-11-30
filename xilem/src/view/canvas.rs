// Copyright 2024 the Xilem Authors
// SPDX-License-Identifier: Apache-2.0

use std::sync::Arc;

use masonry::widgets;
use vello::Scene;
use vello::kurbo::Size;

use crate::core::{Arg, MessageCtx, MessageResult, Mut, View, ViewArgument, ViewMarker};
use crate::{Pod, ViewCtx};

/// # Example
///
/// ```
/// use xilem::masonry::{kurbo::{Rect, Size}, peniko::Fill, vello::Scene};
/// use xilem::{Affine, view::canvas};
/// use std::sync::Arc;
/// # use xilem::WidgetView;
///
/// # fn fill_canvas() -> impl WidgetView<()> + use<> {
/// let my_canvas = canvas(Arc::new(|scene: &mut Scene, size: Size| {
///     // Drawing a simple rectangle that fills the canvas.
///     scene.fill(
///         Fill::NonZero,
///         Affine::IDENTITY,
///         xilem::palette::css::AQUA,
///         None,
///         &Rect::new(0.0, 0.0, size.width, size.height),
///     );
/// }));
/// # my_canvas
/// # }
/// ```
pub fn canvas(draw: Arc<dyn Fn(&mut Scene, Size) + Send + Sync + 'static>) -> Canvas {
    Canvas {
        draw,
        alt_text: None,
    }
}

/// The [`View`] created by [`canvas`].
#[must_use = "View values do nothing unless provided to Xilem."]
pub struct Canvas {
    draw: Arc<dyn Fn(&mut Scene, Size) + Send + Sync + 'static>,
    alt_text: Option<String>,
}

impl Canvas {
    /// Sets alt text for the contents of the canvas.
    ///
    /// Users are strongly encouraged to provide alt text for accessibility tools
    /// to use.
    pub fn alt_text(mut self, alt_text: String) -> Self {
        self.alt_text = Some(alt_text);
        self
    }
}

impl ViewMarker for Canvas {}

impl<State: ViewArgument, Action> View<State, Action, ViewCtx> for Canvas {
    type Element = Pod<widgets::Canvas>;
    type ViewState = ();

    fn build(&self, ctx: &mut ViewCtx, _: Arg<'_, State>) -> (Self::Element, Self::ViewState) {
        let mut widget = widgets::Canvas::from_arc(self.draw.clone());

        if let Some(alt_text) = &self.alt_text {
            widget = widget.with_alt_text(alt_text.to_owned());
        }

        let widget_pod = ctx.create_pod(widget);
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
        if !Arc::ptr_eq(&self.draw, &prev.draw) {
            widgets::Canvas::set_painter_arc(&mut element, self.draw.clone());
        }
        if self.alt_text != prev.alt_text
            && let Some(alt_text) = &self.alt_text
        {
            widgets::Canvas::set_alt_text(element, alt_text.clone());
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
