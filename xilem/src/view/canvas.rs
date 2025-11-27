// Copyright 2024 the Xilem Authors
// SPDX-License-Identifier: Apache-2.0

use std::sync::Arc;

use masonry::core::WidgetMut;
use masonry::widgets;
use vello::Scene;
use vello::kurbo::Size;
use xilem_core::MessageContext;

use crate::core::{Mut, View, ViewMarker};
use crate::{MessageResult, Pod, ViewCtx};

/// Creates a non-interactive drawing surface.
///
/// The `canvas` function provides a way to render custom graphics using a
/// user-supplied drawing callback.
///
/// # Example
///
/// ```
/// use xilem::view::canvas;
/// use xilem::vello::{
///     kurbo::{Affine, Rect},
///     peniko::{Color, Fill},
///     Scene,
/// };
///
/// let my_canvas = canvas(|scene: &mut Scene, size| {
///     // Define a rectangle that fills the entire canvas.
///     let rect = Rect::new(0.0, 0.0, size.width, size.height);
///
///     // Fill the rectangle with a solid color.
///     scene.fill(
///         Fill::NonZero,
///         Affine::IDENTITY,
///         &Color::from_rgb8(51, 102, 204),
///         None,
///         &rect,
///     );
/// });
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
impl<State, Action> View<State, Action, ViewCtx> for Canvas {
    type Element = Pod<widgets::Canvas>;
    type ViewState = ();

    fn build(&self, ctx: &mut ViewCtx, _state: &mut State) -> (Self::Element, Self::ViewState) {
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
        _state: &mut State,
    ) {
        if !Arc::ptr_eq(&self.draw, &prev.draw) {
            widgets::Canvas::set_painter_arc(&mut element, self.draw.clone());
        }
        if self.alt_text != prev.alt_text {
            element.set_alt_text(&mut self.alt_text.clone());
        }
    }

    fn teardown(&self, (): &mut Self::ViewState, _: &mut ViewCtx, _: Mut<'_, Self::Element>) {}

    fn message(
        &self,
        (): &mut Self::ViewState,
        _ctx: &mut MessageContext,
        _widget: WidgetMut<'_, widgets::Canvas>,
        _app_state: &mut State,
    ) -> MessageResult<Action> {
        tracing::error!(
            "Message arrived in Canvas::message, but Canvas doesn't consume any messages, this is a bug"
        );
        MessageResult::Stale
    }
}
