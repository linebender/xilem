// Copyright 2024 the Xilem Authors
// SPDX-License-Identifier: Apache-2.0

use masonry::{widget, Color};
use xilem_core::{Mut, ViewMarker};

use crate::{MessageResult, Pod, View, ViewCtx, ViewId};

pub fn spinner() -> Spinner {
    Spinner { color: None }
}

pub struct Spinner {
    color: Option<Color>,
}

impl Spinner {
    pub fn color(mut self, color: Color) -> Self {
        self.color = Some(color);
        self
    }
}

impl ViewMarker for Spinner {}
impl<State, Action> View<State, Action, ViewCtx> for Spinner {
    type Element = Pod<widget::Spinner>;
    type ViewState = ();

    fn build(&self, _: &mut ViewCtx) -> (Self::Element, Self::ViewState) {
        (Pod::new(masonry::widget::Spinner::new()), ())
    }

    fn rebuild<'el>(
        &self,
        prev: &Self,
        (): &mut Self::ViewState,
        _: &mut ViewCtx,
        mut element: Mut<'el, Self::Element>,
    ) -> Mut<'el, Self::Element> {
        if prev.color != self.color {
            // TODO: Don't duplicate the default colour from Masonry
            element.set_color(self.color.unwrap_or(masonry::theme::TEXT_COLOR));
        }
        element
    }

    fn teardown(&self, (): &mut Self::ViewState, _: &mut ViewCtx, _: Mut<'_, Self::Element>) {}

    fn message(
        &self,
        (): &mut Self::ViewState,
        _: &[ViewId],
        message: xilem_core::DynMessage,
        _: &mut State,
    ) -> MessageResult<Action> {
        tracing::error!("Message arrived in Label::message, but Label doesn't consume any messages, this is a bug");
        MessageResult::Stale(message)
    }
}
