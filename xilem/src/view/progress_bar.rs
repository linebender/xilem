// Copyright 2024 the Xilem Authors
// SPDX-License-Identifier: Apache-2.0

use masonry::widget;
use xilem_core::{Mut, ViewMarker};

use crate::{MessageResult, Pod, View, ViewCtx, ViewId};

pub fn progress_bar(part_complete: Option<f32>) -> ProgressBar {
    ProgressBar { part_complete }
}

pub struct ProgressBar {
    part_complete: Option<f32>,
}

impl ViewMarker for ProgressBar {}
impl<State, Action> View<State, Action, ViewCtx> for ProgressBar {
    type Element = Pod<widget::ProgressBar>;
    type ViewState = ();

    fn build(&self, ctx: &mut ViewCtx) -> (Self::Element, Self::ViewState) {
        ctx.with_leaf_action_widget(|_| {
            Pod::new(masonry::widget::ProgressBar::new(self.part_complete))
        })
    }

    fn rebuild<'el>(
        &self,
        prev: &Self,
        (): &mut Self::ViewState,
        ctx: &mut ViewCtx,
        mut element: Mut<'el, Self::Element>,
    ) -> Mut<'el, Self::Element> {
        if prev.part_complete != self.part_complete {
            element.set_part_complete(self.part_complete);
            ctx.mark_changed();
        }
        element
    }

    fn teardown(
        &self,
        (): &mut Self::ViewState,
        ctx: &mut ViewCtx,
        element: Mut<'_, Self::Element>,
    ) {
        ctx.teardown_leaf(element);
    }

    fn message(
        &self,
        (): &mut Self::ViewState,
        id_path: &[ViewId],
        message: xilem_core::DynMessage,
        _app_state: &mut State,
    ) -> MessageResult<Action> {
        debug_assert!(
            id_path.is_empty(),
            "id path should be empty in ProgressBar::message"
        );
        match message.downcast::<masonry::Action>() {
            Ok(_) => MessageResult::Nop,
            Err(message) => {
                tracing::error!("Wrong message type in Checkbox::message");
                MessageResult::Stale(message)
            }
        }
    }
}
