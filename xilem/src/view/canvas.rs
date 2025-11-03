// Copyright 2024 the Xilem Authors
// SPDX-License-Identifier: Apache-2.0

use std::sync::Arc;

use masonry::core::WidgetMut;
use masonry::widgets;
use vello::Scene;
use vello::kurbo::Size;
use xilem_core::MessageContext;

use crate::core::{Mut, ViewMarker};
use crate::{MessageResult, Pod, View, ViewCtx};

/// Creates a non-interactive drawing surface.
///
/// The `canvas` function provides a way to render custom graphics using a user-
/// supplied drawing callback. The callback receives a mutable reference to a
/// `Scene` and the current allocated `Size` of the canvas, allowing you to draw
/// shapes, images, or any other custom content.
///
/// # Example
///
/// ```ignore
/// use xilem::view::canvas;
/// use masonry::{Scene, Size};
///
/// let my_canvas = canvas(|scene: &mut Scene, size: Size| {
///     // Drawing a simple rectangle that fills the canvas.
///     scene.fill_rect((0.0, 0.0, size.width, size.height), [0.2, 0.4, 0.8, 1.0]);
/// });
/// ```
pub fn canvas(draw: impl Fn(&mut Scene, Size) + Send + Sync + 'static) -> Canvas {
    Canvas {
        draw: Arc::new(draw),
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
        let widget = widgets::Canvas::from_arc(self.draw.clone());

        let widget_pod = ctx.create_pod(widget);
        (widget_pod, ())
    }
    fn rebuild(
        &self,
        prev: &Self,
        (): &mut Self::ViewState,
        _ctx: &mut ViewCtx,
        element: Mut<'_, Self::Element>,
        _state: &mut State,
    ) {
        if !Arc::ptr_eq(&self.draw, &prev.draw) {
            widgets::Canvas::set_painter_arc(element, self.draw.clone());
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
