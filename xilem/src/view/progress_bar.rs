// Copyright 2024 the Xilem Authors
// SPDX-License-Identifier: Apache-2.0

use masonry::widget;

use crate::core::{DynMessage, Mut, ViewMarker};
use crate::{Affine, MessageResult, Pod, View, ViewCtx, ViewId};

use super::Transformable;

pub fn progress_bar(progress: Option<f64>) -> ProgressBar {
    ProgressBar {
        progress,
        transform: Affine::IDENTITY,
    }
}

pub struct ProgressBar {
    progress: Option<f64>,
    transform: Affine,
}

impl Transformable for ProgressBar {
    fn transform_mut(&mut self) -> &mut Affine {
        &mut self.transform
    }
}

impl ViewMarker for ProgressBar {}
impl<State, Action> View<State, Action, ViewCtx> for ProgressBar {
    type Element = Pod<widget::ProgressBar>;
    type ViewState = ();

    fn build(&self, ctx: &mut ViewCtx) -> (Self::Element, Self::ViewState) {
        ctx.with_leaf_action_widget(|ctx| {
            ctx.new_pod_with_transform(widget::ProgressBar::new(self.progress), self.transform)
        })
    }

    fn rebuild(
        &self,
        prev: &Self,
        (): &mut Self::ViewState,
        _ctx: &mut ViewCtx,
        mut element: Mut<Self::Element>,
    ) {
        if prev.transform != self.transform {
            element.set_transform(self.transform);
        }
        if prev.progress != self.progress {
            widget::ProgressBar::set_progress(&mut element, self.progress);
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
        tracing::error!("Message arrived in ProgressBar::message, but ProgressBar doesn't consume any messages, this is a bug");
        MessageResult::Stale(message)
    }
}
