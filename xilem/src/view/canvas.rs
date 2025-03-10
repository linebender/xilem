// Copyright 2024 the Xilem Authors
// SPDX-License-Identifier: Apache-2.0

use std::sync::Arc;

use masonry::widgets;
use vello::kurbo::Size;
use vello::Scene;

use crate::core::{DynMessage, Mut, ViewMarker};
use crate::{MessageResult, Pod, View, ViewCtx, ViewId};

/// A non-interactive text element.
/// # Example
///
/// ```ignore
/// use xilem::palette;
/// use xilem::view::label;
/// use masonry::TextAlignment;
/// use masonry::parley::fontique;
///
/// label("Text example.")
///     .brush(palette::css::RED)
///     .alignment(TextAlignment::Middle)
///     .text_size(24.0)
///     .weight(FontWeight::BOLD)
///     .with_font(fontique::GenericFamily::Serif)
/// ```
pub fn canvas(draw: impl Fn(&mut Scene, Size) + Send + Sync + 'static) -> Canvas {
    Canvas {
        draw: Arc::new(draw),
        alt_text: None,
    }
}

/// Create a canvas view.
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

    fn build(&self, ctx: &mut ViewCtx) -> (Self::Element, Self::ViewState) {
        let widget = widgets::Canvas::from_arc(self.draw.clone());

        let widget_pod = ctx.new_pod(widget);
        (widget_pod, ())
    }

    fn rebuild(
        &self,
        prev: &Self,
        (): &mut Self::ViewState,
        _ctx: &mut ViewCtx,
        element: Mut<Self::Element>,
    ) {
        if !Arc::ptr_eq(&self.draw, &prev.draw) {
            widgets::Canvas::set_painter_arc(element, self.draw.clone());
        }
    }

    fn teardown(&self, (): &mut Self::ViewState, _: &mut ViewCtx, _: Mut<Self::Element>) {}

    fn message(
        &self,
        (): &mut Self::ViewState,
        _id_path: &[ViewId],
        message: DynMessage,
        _app_state: &mut State,
    ) -> MessageResult<Action> {
        tracing::error!("Message arrived in Canvas::message, but Canvas doesn't consume any messages, this is a bug");
        MessageResult::Stale(message)
    }
}
