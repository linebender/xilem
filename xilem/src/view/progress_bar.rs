// Copyright 2024 the Xilem Authors
// SPDX-License-Identifier: Apache-2.0

use masonry::widgets;

use crate::core::{DynMessage, MessageResult, Mut, View, ViewId, ViewMarker};
use crate::{Pod, ViewCtx};

/// A view which displays a progress bar.
///
/// This can be for showing progress of a task or a download.
pub fn progress_bar(progress: Option<f64>) -> ProgressBar {
    ProgressBar { progress }
}

/// The [`View`] created by [`progress_bar`].
#[must_use = "View values do nothing unless provided to Xilem."]
pub struct ProgressBar {
    progress: Option<f64>,
}

impl ViewMarker for ProgressBar {}
impl<State, Action> View<State, Action, ViewCtx> for ProgressBar {
    type Element = Pod<widgets::ProgressBar>;
    type ViewState = ();

    fn build(&self, ctx: &mut ViewCtx) -> (Self::Element, Self::ViewState) {
        ctx.with_leaf_action_widget(|ctx| ctx.create_pod(widgets::ProgressBar::new(self.progress)))
    }

    fn rebuild(
        &self,
        prev: &Self,
        (): &mut Self::ViewState,
        _ctx: &mut ViewCtx,
        mut element: Mut<Self::Element>,
    ) {
        if prev.progress != self.progress {
            widgets::ProgressBar::set_progress(&mut element, self.progress);
        }
    }

    fn teardown(&self, (): &mut Self::ViewState, ctx: &mut ViewCtx, element: Mut<Self::Element>) {
        ctx.teardown_leaf(element);
    }

    fn message(
        &self,
        (): &mut Self::ViewState,
        _id_path: &[ViewId],
        message: DynMessage,
        _app_state: &mut State,
    ) -> MessageResult<Action> {
        tracing::error!(
            "Message arrived in ProgressBar::message, but ProgressBar doesn't consume any messages, this is a bug"
        );
        MessageResult::Stale(message)
    }
}
