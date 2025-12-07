// Copyright 2024 the Xilem Authors
// SPDX-License-Identifier: Apache-2.0

use std::marker::PhantomData;

use masonry::core::ArcStr;
use masonry::widgets::{self, CanvasSizeChanged};
use vello::Scene;
use vello::kurbo::Size;

use crate::core::{Arg, MessageCtx, MessageResult, Mut, View, ViewArgument, ViewMarker};
use crate::{Pod, ViewCtx};

/// Access a raw vello [`Scene`] within a canvas that fills its parent
///
/// # Example
///
/// ```
/// # use xilem_masonry as xilem;
/// use xilem::{view::canvas, masonry::{palette, kurbo::{Rect, Size, Affine}, peniko::Fill, vello::Scene}};
/// # use xilem::{WidgetView, core::Edit};
///
/// # fn fill_canvas<State: 'static>() -> impl WidgetView<Edit<State>> {
/// let my_canvas = canvas(|_state: &mut State, scene: &mut Scene, size: Size| {
///     // Drawing a simple rectangle that fills the canvas.
///     scene.fill(
///         Fill::NonZero,
///         Affine::IDENTITY,
///         palette::css::AQUA,
///         None,
///         &Rect::new(0.0, 0.0, size.width, size.height),
///     );
/// });
/// # my_canvas
/// # }
/// ```
pub fn canvas<State, F>(draw: F) -> Canvas<State, F>
where
    State: ViewArgument,
    F: Fn(Arg<'_, State>, &mut Scene, Size) + Send + Sync + 'static,
{
    Canvas {
        draw,
        alt_text: Option::default(),
        phantom: PhantomData,
    }
}

/// The [`View`] created by [`canvas`].
#[must_use = "View values do nothing unless provided to Xilem."]
pub struct Canvas<State, F> {
    draw: F,
    alt_text: Option<ArcStr>,
    phantom: PhantomData<fn() -> State>,
}

impl<State, F> Canvas<State, F> {
    /// Sets alt text for the contents of the canvas.
    ///
    /// Users are strongly encouraged to provide alt text for accessibility tools
    /// to use.
    pub fn alt_text(mut self, alt_text: impl Into<ArcStr>) -> Self {
        self.alt_text = Some(alt_text.into());
        self
    }
}

impl<State, F> ViewMarker for Canvas<State, F> {}

impl<State, Action, F> View<State, Action, ViewCtx> for Canvas<State, F>
where
    State: ViewArgument,
    F: Fn(Arg<'_, State>, &mut Scene, Size) + Send + Sync + 'static,
{
    type Element = Pod<widgets::Canvas>;
    type ViewState = ();

    fn build(&self, ctx: &mut ViewCtx, _: Arg<'_, State>) -> (Self::Element, Self::ViewState) {
        ctx.with_leaf_action_widget(|ctx| {
            let widget = match &self.alt_text {
                Some(alt_text) => widgets::Canvas::default().with_alt_text(alt_text.clone()),
                None => widgets::Canvas::default(),
            };
            ctx.create_pod(widget)
        })
    }

    fn rebuild(
        &self,
        prev: &Self,
        (): &mut Self::ViewState,
        _ctx: &mut ViewCtx,
        mut element: Mut<'_, Self::Element>,
        state: Arg<'_, State>,
    ) {
        widgets::Canvas::update_scene(&mut element, |scene, size| (self.draw)(state, scene, size));
        if self.alt_text != prev.alt_text {
            widgets::Canvas::set_alt_text(&mut element, self.alt_text.clone());
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
        debug_assert!(
            message.remaining_path().is_empty(),
            "id path should be empty in Canvas::message"
        );
        match message.take_message::<CanvasSizeChanged>() {
            Some(_) => MessageResult::RequestRebuild,
            None => {
                tracing::error!("Wrong message type in Canvas::message, got {message:?}.");
                MessageResult::Stale
            }
        }
    }
}
