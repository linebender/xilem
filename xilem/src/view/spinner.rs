// Copyright 2024 the Xilem Authors
// SPDX-License-Identifier: Apache-2.0

use masonry_winit::peniko::Color;
use masonry_winit::widgets;

use crate::core::{DynMessage, Mut, ViewMarker};
use crate::{MessageResult, Pod, View, ViewCtx, ViewId};

/// An indefinite spinner.
///
/// This can be used to display that progress is happening on some process,
/// but that the exact status is not known.
///
/// The underlying widget is the Masonry [`Spinner`](widgets::Spinner).
///
/// # Examples
///
/// ```rust,no_run
/// # use xilem::{view::{spinner, flex}, WidgetView, core::one_of::Either};
/// # struct ApiClient;
/// # struct RequestState { pending: bool }
/// # impl RequestState {
/// #     fn request_result(&mut self) -> impl WidgetView<ApiClient> { flex(()) }
/// # }
/// #
/// fn show_request_outcome(data: &mut RequestState) -> impl WidgetView<ApiClient>  {
///     if data.pending {
///         Either::A(spinner())
///     } else {
///         Either::B(data.request_result())
///     }
/// }
/// ```
pub fn spinner() -> Spinner {
    Spinner { color: None }
}

/// The [`View`] created by [`spinner`].
///
/// See `spinner`'s docs for more details.
#[must_use = "View values do nothing unless provided to Xilem."]
pub struct Spinner {
    color: Option<Color>,
}

impl Spinner {
    /// Set the color for this spinner.
    pub fn color(mut self, color: impl Into<Color>) -> Self {
        self.color = Some(color.into());
        self
    }
}

impl ViewMarker for Spinner {}
impl<State, Action> View<State, Action, ViewCtx> for Spinner {
    type Element = Pod<widgets::Spinner>;
    type ViewState = ();

    fn build(&self, ctx: &mut ViewCtx) -> (Self::Element, Self::ViewState) {
        let pod = ctx.new_pod(widgets::Spinner::new());
        (pod, ())
    }

    fn rebuild(
        &self,
        prev: &Self,
        (): &mut Self::ViewState,
        _: &mut ViewCtx,
        mut element: Mut<Self::Element>,
    ) {
        if prev.color != self.color {
            match self.color {
                Some(color) => widgets::Spinner::set_color(&mut element, color),
                None => widgets::Spinner::reset_color(&mut element),
            };
        }
    }

    fn teardown(&self, (): &mut Self::ViewState, _: &mut ViewCtx, _: Mut<Self::Element>) {}

    fn message(
        &self,
        (): &mut Self::ViewState,
        _: &[ViewId],
        message: DynMessage,
        _: &mut State,
    ) -> MessageResult<Action> {
        tracing::error!(
            "Message arrived in Spinner::message, but Spinner doesn't consume any messages, this is a bug"
        );
        MessageResult::Stale(message)
    }
}
