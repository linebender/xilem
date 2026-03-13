// Copyright 2025 the Xilem Authors
// SPDX-License-Identifier: Apache-2.0

use alloc::boxed::Box;
use core::ops::Deref;

use crate::message::MessageResult;
use crate::{MessageCtx, Mut, View, ViewMarker, ViewPathTracker};

impl<V: ?Sized> ViewMarker for Box<V> {}
impl<State, Action, Context, V> View<State, Action, Context> for Box<V>
where
    State: 'static,
    Context: ViewPathTracker,
    V: View<State, Action, Context> + ?Sized,
{
    type Element = V::Element;
    type ViewState = V::ViewState;

    fn build(&self, ctx: &mut Context, app_state: &mut State) -> (Self::Element, Self::ViewState) {
        self.deref().build(ctx, app_state)
    }

    fn rebuild(
        &self,
        prev: &Self,
        view_state: &mut Self::ViewState,
        ctx: &mut Context,
        element: Mut<'_, Self::Element>,
        app_state: &mut State,
    ) {
        self.deref()
            .rebuild(prev, view_state, ctx, element, app_state);
    }

    fn teardown(
        &self,
        view_state: &mut Self::ViewState,
        ctx: &mut Context,
        element: Mut<'_, Self::Element>,
    ) {
        self.deref().teardown(view_state, ctx, element);
    }

    fn message(
        &self,
        view_state: &mut Self::ViewState,
        message: &mut MessageCtx,
        element: Mut<'_, Self::Element>,
        app_state: &mut State,
    ) -> MessageResult<Action> {
        self.deref()
            .message(view_state, message, element, app_state)
    }
}
