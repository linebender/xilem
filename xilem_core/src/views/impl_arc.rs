// Copyright 2025 the Xilem Authors
// SPDX-License-Identifier: Apache-2.0

use alloc::sync::Arc;
use core::ops::Deref;

use crate::message::MessageResult;
use crate::views::impl_rc::RcState;
use crate::{Arg, MessageContext, Mut, View, ViewArgument, ViewMarker, ViewPathTracker};

impl<V: ?Sized> ViewMarker for Arc<V> {}
/// An implementation of [`View`] which only runs rebuild if the states are different
impl<State, Action, Context, V> View<State, Action, Context> for Arc<V>
where
    State: ViewArgument,
    Context: ViewPathTracker,
    V: View<State, Action, Context> + ?Sized,
{
    type Element = V::Element;
    type ViewState = RcState<V::ViewState>;

    fn build(
        &self,
        ctx: &mut Context,
        app_state: Arg<'_, State>,
    ) -> (Self::Element, Self::ViewState) {
        let (element, view_state) = self.deref().build(ctx, app_state);
        (
            element,
            RcState {
                view_state,
                dirty: false,
            },
        )
    }

    fn rebuild(
        &self,
        prev: &Self,
        view_state: &mut Self::ViewState,
        ctx: &mut Context,
        element: Mut<'_, Self::Element>,
        app_state: Arg<'_, State>,
    ) {
        #![expect(clippy::use_self, reason = "`Arc::ptr_eq` is the canonical form")]
        if core::mem::take(&mut view_state.dirty) || !Arc::ptr_eq(self, prev) {
            self.deref()
                .rebuild(prev, &mut view_state.view_state, ctx, element, app_state);
        }
    }

    fn teardown(
        &self,
        view_state: &mut Self::ViewState,
        ctx: &mut Context,
        element: Mut<'_, Self::Element>,
    ) {
        self.deref()
            .teardown(&mut view_state.view_state, ctx, element);
    }

    fn message(
        &self,
        view_state: &mut Self::ViewState,
        message: &mut MessageContext,
        element: Mut<'_, Self::Element>,
        app_state: Arg<'_, State>,
    ) -> MessageResult<Action> {
        let message_result =
            self.deref()
                .message(&mut view_state.view_state, message, element, app_state);
        if matches!(message_result, MessageResult::RequestRebuild) {
            view_state.dirty = true;
        }
        message_result
    }
}
