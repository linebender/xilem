// Copyright 2024 the Xilem Authors
// SPDX-License-Identifier: Apache-2.0

use masonry::widgets;

use crate::core::{MessageCtx, MessageResult, Mut, View, ViewMarker};
use crate::{Pod, ViewCtx};

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
/// # use xilem_masonry as xilem;
/// # use xilem::{view::{spinner, flex_col}, WidgetView, core::one_of::Either};
/// # struct ApiClient;
/// # use xilem::core::Edit;
/// # struct RequestState { pending: bool }
/// # impl RequestState {
/// #     fn request_result(&mut self) -> impl WidgetView<Edit<ApiClient>> { flex_col(()) }
/// # }
/// #
/// fn show_request_outcome(data: &mut RequestState) -> impl WidgetView<Edit<ApiClient>>  {
///     if data.pending {
///         Either::A(spinner())
///     } else {
///         Either::B(data.request_result())
///     }
/// }
/// ```
pub fn spinner() -> Spinner {
    Spinner
}

/// The [`View`] created by [`spinner`].
///
/// See `spinner`'s docs for more details.
#[must_use = "View values do nothing unless provided to Xilem."]
pub struct Spinner;

impl ViewMarker for Spinner {}
impl<State: 'static, Action> View<State, Action, ViewCtx> for Spinner {
    type Element = Pod<widgets::Spinner>;
    type ViewState = ();

    fn build(&self, ctx: &mut ViewCtx, _: &mut State) -> (Self::Element, Self::ViewState) {
        (ctx.create_pod(widgets::Spinner::new()), ())
    }

    fn rebuild(
        &self,
        _: &Self,
        (): &mut Self::ViewState,
        _: &mut ViewCtx,
        _: Mut<'_, Self::Element>,
        _: &mut State,
    ) {
    }

    fn teardown(&self, (): &mut Self::ViewState, _: &mut ViewCtx, _: Mut<'_, Self::Element>) {}

    fn message(
        &self,
        (): &mut Self::ViewState,
        message: &mut MessageCtx,
        _element: Mut<'_, Self::Element>,
        _: &mut State,
    ) -> MessageResult<Action> {
        tracing::error!(
            ?message,
            "Message arrived in Spinner::message, but Spinner doesn't consume any messages, this is a bug"
        );
        MessageResult::Stale
    }
}
